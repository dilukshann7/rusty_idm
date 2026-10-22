use crate::{DestinationIdentity, ErrorKind, JobError, ReplacementPolicy};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
};

static NEXT_PARTIAL: AtomicU64 = AtomicU64::new(1);

pub fn inspect_destination(path: &Path) -> Result<Option<DestinationIdentity>, JobError> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => {
                return Err(JobError::new(
                    ErrorKind::Io,
                    "Cannot open the existing destination for inspection.",
                ))
            }
        };
        return identify_destination(&file).map(Some);
    }
    #[cfg(not(windows))]
    {
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => {
                return Err(JobError::new(
                    ErrorKind::Io,
                    "Cannot inspect the destination. Check the folder and permissions.",
                ))
            }
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(JobError::new(
                ErrorKind::DestinationConflict,
                "The destination is not a regular file. Choose a different filename.",
            ));
        }
        Ok(Some(DestinationIdentity {
            size: metadata.len(),
            modified: metadata.modified().ok(),
        }))
    }
}

#[cfg(windows)]
pub(crate) fn identify_destination(file: &std::fs::File) -> Result<DestinationIdentity, JobError> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY,
            FILE_ATTRIBUTE_REPARSE_POINT,
        },
    };
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info) }
        .map_err(|_| JobError::new(ErrorKind::Io, "Cannot identify the existing destination."))?;
    if info.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY.0 | FILE_ATTRIBUTE_REPARSE_POINT.0) != 0 {
        return Err(JobError::new(
            ErrorKind::DestinationConflict,
            "The destination is not a regular file. Choose a different filename.",
        ));
    }
    let metadata = file.metadata().map_err(|_| {
        JobError::new(
            ErrorKind::Io,
            "Cannot inspect the existing destination handle.",
        )
    })?;
    Ok(DestinationIdentity {
        size: ((info.nFileSizeHigh as u64) << 32) | info.nFileSizeLow as u64,
        modified: metadata.modified().ok(),
        volume: info.dwVolumeSerialNumber,
        index: ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
    })
}

#[cfg(windows)]
fn guard_destination(
    destination: &Path,
    expected: &DestinationIdentity,
) -> Result<std::fs::File, JobError> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows::Win32::{
        Foundation::GENERIC_READ,
        Storage::FileSystem::{DELETE, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ},
    };
    // Deny writers and renames until the approved original is committed or released.
    // OPEN_REPARSE_POINT ensures a substituted symlink is inspected, never followed.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .access_mode(GENERIC_READ.0 | DELETE.0)
        .share_mode(FILE_SHARE_READ.0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(destination)
        .map_err(|_| JobError::new(ErrorKind::DestinationChanged, "Cannot guard the approved destination. It may have changed or be open for writing; choose the file again."))?;
    if identify_destination(&file)? != *expected {
        return Err(JobError::new(
            ErrorKind::DestinationChanged,
            "The approved destination has changed. Choose the file again before replacing it.",
        ));
    }
    Ok(file)
}

/// Owns its unique partial and guards an approved original throughout the transfer.
/// Drop cleans up only the partial, never the original or a completed destination.
pub struct OutputFile {
    file: Option<File>,
    partial: PathBuf,
    destination: PathBuf,
    retain: bool,
    recovery: Option<crate::storage::Recovery>,
    total: Option<u64>,
    response_url: Option<String>,
    etag: Option<String>,
    refresh_pending: bool,
    creation_time: Option<std::time::SystemTime>,
    #[cfg(windows)]
    original: Option<std::fs::File>,
}
impl OutputFile {
    pub async fn prepare(
        destination: &Path,
        replacement: &ReplacementPolicy,
    ) -> Result<Self, JobError> {
        Self::prepare_in(destination, replacement, None).await
    }
    async fn prepare_in(
        destination: &Path,
        replacement: &ReplacementPolicy,
        temporary: Option<&Path>,
    ) -> Result<Self, JobError> {
        #[cfg(windows)]
        let original = match replacement {
            ReplacementPolicy::Approved(expected) => {
                Some(guard_destination(destination, expected)?)
            }
            ReplacementPolicy::Deny => None,
        };
        #[cfg(not(windows))]
        if matches!(replacement, ReplacementPolicy::Approved(_)) {
            return Err(JobError::new(
                ErrorKind::DestinationConflict,
                "Approved replacement is supported only on Windows. Choose a new filename.",
            ));
        }
        if matches!(replacement, ReplacementPolicy::Deny) {
            match tokio::fs::symlink_metadata(destination).await {
                Ok(_) => {
                    return Err(JobError::new(
                        ErrorKind::DestinationConflict,
                        "The destination already exists. Choose a new filename.",
                    ))
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => {
                    return Err(JobError::new(
                        ErrorKind::Io,
                        "Cannot inspect the destination folder.",
                    ))
                }
            }
        }
        let parent = temporary.unwrap_or_else(|| {
            destination
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."))
        });
        if temporary.is_some() {
            tokio::fs::create_dir_all(parent).await.map_err(|_| {
                JobError::new(
                    ErrorKind::Io,
                    "Cannot create/open the configured temporary directory.",
                )
            })?;
        }
        for _ in 0..128 {
            // Fixed short partial names also work beside very long final filenames.
            let id = NEXT_PARTIAL.fetch_add(1, Ordering::Relaxed);
            let partial = parent.join(format!(".rusty-{}-{id}.part", std::process::id()));
            let mut options = OpenOptions::new();
            options.read(true).write(true).create_new(true);
            #[cfg(windows)]
            {
                use windows::Win32::{
                    Foundation::{GENERIC_READ, GENERIC_WRITE},
                    Storage::FileSystem::{DELETE, FILE_SHARE_READ},
                };
                options
                    .access_mode(GENERIC_READ.0 | GENERIC_WRITE.0 | DELETE.0)
                    .share_mode(FILE_SHARE_READ.0);
            }
            match options.open(&partial).await {
                Ok(file) => return Ok(Self { file: Some(file), partial, destination: destination.to_owned(), retain: false, recovery: None, total: None, response_url: None, etag: None, refresh_pending: false, creation_time: None, #[cfg(windows)] original }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(JobError::new(ErrorKind::Io, "Cannot reserve a partial file. Check the save folder, permissions and available space.")),
            }
        }
        Err(JobError::new(
            ErrorKind::Io,
            "Cannot reserve a unique partial filename.",
        ))
    }
    pub(crate) async fn durable_prepare(
        request: &crate::DownloadRequest,
    ) -> Result<Self, JobError> {
        if let Some(recovery) = &request.recovery {
            if let Some(checkpoint) = recovery.checkpoint() {
                for segment in &checkpoint.segments {
                    if checkpoint.cleanup_pending.contains(&segment.partial)
                        && checkpoint.total == Some(checkpoint.durable)
                        && !segment.partial.exists()
                    {
                        continue;
                    }
                    verify_segment(segment, request.options.ignore_modified_time).map_err(|e| {
                        JobError::new(ErrorKind::RestartRequired,
                            format!("Restart required: saved range verification failed: {e}. All recorded files were preserved."))
                            .retained(segment.partial.clone())
                    })?;
                }
                let file = reopen_verified(&checkpoint, request.options.ignore_modified_time)
                    .map_err(|e| {
                        JobError::new(
                            ErrorKind::RestartRequired,
                            format!("Restart required: {e}. The recorded file was preserved."),
                        )
                        .retained(checkpoint.partial.clone())
                    })?;
                #[cfg(windows)]
                let original = match &request.replacement {
                    ReplacementPolicy::Approved(expected) => {
                        Some(guard_destination(&request.destination, expected)?)
                    }
                    ReplacementPolicy::Deny => None,
                };
                // Restored approvals never survive restart. Do not overwrite an existing file.
                if matches!(request.replacement, ReplacementPolicy::Deny)
                    && request.destination.exists()
                {
                    return Err(JobError::new(ErrorKind::RestartRequired,"Restart required: the destination exists and requires fresh overwrite approval.").retained(checkpoint.partial));
                }
                return Ok(Self {
                    file: Some(File::from_std(file)),
                    partial: checkpoint.partial,
                    destination: request.destination.clone(),
                    retain: true,
                    recovery: Some(recovery.clone()),
                    total: checkpoint.total,
                    response_url: checkpoint.response_url,
                    etag: checkpoint.etag,
                    refresh_pending: checkpoint.refresh_pending,
                    creation_time: checkpoint.creation_time,
                    #[cfg(windows)]
                    original,
                });
            }
        }
        let mut output = Self::prepare_in(
            &request.destination,
            &request.replacement,
            request.options.temporary_directory.as_deref(),
        )
        .await?;
        output.recovery = request.recovery.clone();
        if output.recovery.is_some() {
            output.retain = true;
            output.checkpoint().await?;
        }
        Ok(output)
    }
    pub(crate) fn adopt_recovery(&mut self, recovery: Option<crate::storage::Recovery>) -> bool {
        if self.recovery.is_none() && recovery.is_some() {
            self.recovery = recovery;
            self.retain = true;
            return true;
        }
        false
    }
    pub(crate) fn representation(
        &mut self,
        total: Option<u64>,
        url: Option<String>,
        etag: Option<String>,
    ) {
        self.total = total;
        self.response_url = url.and_then(|s| url::Url::parse(&s).ok()).map(|mut u| {
            let _ = u.set_username("");
            let _ = u.set_password(None);
            u.into()
        });
        self.etag = etag;
    }
    pub(crate) fn clear_refresh_pending(&mut self) {
        self.refresh_pending = false;
    }
    pub(crate) async fn refresh_samples(
        &mut self,
        received: u64,
    ) -> Result<Vec<(u64, Vec<u8>)>, JobError> {
        let file = self.file.as_mut().expect("open partial");
        file.flush().await.map_err(|_| {
            JobError::new(ErrorKind::Io, "Cannot flush saved bytes for verification.")
        })?;
        let previous = file
            .stream_position()
            .await
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot inspect saved byte position."))?;
        let result = async {
            let mut samples = Vec::new();
            for (start, end) in crate::refresh::sample_ranges(received) {
                file.seek(std::io::SeekFrom::Start(start))
                    .await
                    .map_err(|_| JobError::new(ErrorKind::Io, "Cannot seek saved bytes."))?;
                let mut bytes = vec![0; (end - start + 1) as usize];
                file.read_exact(&mut bytes)
                    .await
                    .map_err(|_| JobError::new(ErrorKind::Io, "Cannot read saved bytes."))?;
                samples.push((start, bytes));
            }
            Ok(samples)
        }
        .await;
        file.seek(std::io::SeekFrom::Start(previous))
            .await
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot restore saved byte position."))?;
        result
    }
    pub(crate) async fn checkpoint(&mut self) -> Result<(), JobError> {
        self.checkpoint_inner(None).await.map_err(|error| {
            crate::storage::storage_error(error.message).retained(self.partial.clone())
        })
    }
    pub(crate) async fn checkpoint_ranges(
        &mut self,
        segments: Vec<crate::storage::SegmentCheckpoint>,
        last_modified: Option<String>,
        cleanup_pending: Vec<PathBuf>,
    ) -> Result<(), JobError> {
        self.checkpoint_inner(Some((segments, last_modified, cleanup_pending)))
            .await
            .map_err(|error| {
                crate::storage::storage_error(error.message).retained(self.partial.clone())
            })
    }
    async fn checkpoint_inner(
        &mut self,
        range_map: Option<(
            Vec<crate::storage::SegmentCheckpoint>,
            Option<String>,
            Vec<PathBuf>,
        )>,
    ) -> Result<(), JobError> {
        let Some(recovery) = self.recovery.clone() else {
            return Ok(());
        };
        let file = self.file.as_mut().expect("open partial");
        file.flush()
            .await
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot flush queued partial."))?;
        file.sync_all()
            .await
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot sync queued partial."))?;
        let file = file
            .try_clone()
            .await
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot inspect queued partial."))?
            .into_std()
            .await;
        let path = self.partial.clone();
        let total = self.total;
        let response_url = self.response_url.clone();
        let etag = self.etag.clone();
        let refresh_pending = self.refresh_pending;
        let mut checkpoint = tokio::task::spawn_blocking(move || {
            make_checkpoint(file, path, total, response_url, etag)
        })
        .await
        .map_err(|_| JobError::new(ErrorKind::Io, "Checkpoint worker failed."))??;
        checkpoint.creation_time = self.creation_time;
        checkpoint.refresh_pending = refresh_pending;
        let (segments, last_modified, cleanup_pending) = range_map.unwrap_or_else(|| {
            recovery
                .checkpoint()
                .map(|old| (old.segments, old.last_modified, old.cleanup_pending))
                .unwrap_or_default()
        });
        checkpoint.segments = segments;
        checkpoint.last_modified = last_modified;
        checkpoint.cleanup_pending = cleanup_pending;
        recovery
            .save(checkpoint)
            .map_err(|e| e.retained(self.partial.clone()))
    }
    pub(crate) fn preserved_error(&self, mut error: JobError) -> JobError {
        error.retained_partial = Some(self.partial.clone());
        error
    }
    pub fn partial_path(&self) -> &Path {
        &self.partial
    }
    pub(crate) async fn length(&self) -> Result<u64, JobError> {
        self.file
            .as_ref()
            .expect("open partial")
            .metadata()
            .await
            .map(|m| m.len())
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot inspect the partial length."))
    }
    pub(crate) async fn seek_end(&mut self) -> Result<(), JobError> {
        self.file
            .as_mut()
            .expect("open partial")
            .seek(std::io::SeekFrom::End(0))
            .await
            .map(|_| ())
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot seek the partial."))
    }
    /// Restart only the owned open partial, never a separately reopened path.
    pub async fn restart(&mut self) -> Result<(), JobError> {
        let file = self.file.as_mut().expect("open partial");
        file.flush().await.map_err(|_| {
            JobError::new(ErrorKind::Io, "Cannot flush the partial before restarting.")
        })?;
        file.set_len(0).await.map_err(|_| {
            JobError::new(
                ErrorKind::Io,
                "Cannot truncate the partial before restarting.",
            )
        })?;
        file.rewind().await.map_err(|_| {
            JobError::new(
                ErrorKind::Io,
                "Cannot rewind the partial before restarting.",
            )
        })?;
        Ok(())
    }
    pub async fn write(&mut self, bytes: &[u8]) -> Result<(), JobError> {
        self.file
            .as_mut()
            .expect("open partial")
            .write_all(bytes)
            .await
            .map_err(|_| {
                JobError::new(
                    ErrorKind::Io,
                    "Writing the download failed. Check free space and permissions.",
                )
            })
    }
    pub async fn abort(mut self) -> Result<(), JobError> {
        self.retain = true;
        if let Some(mut checkpoint) = self.recovery.as_ref().and_then(|r| r.checkpoint()) {
            checkpoint.cleanup_pending = checkpoint
                .segments
                .iter()
                .map(|s| s.partial.clone())
                .collect();
            if let Some(recovery) = &self.recovery {
                recovery.save(checkpoint.clone())?;
            }
            for segment in &checkpoint.segments {
                let file =
                    match verify_segment(segment, true) {
                        Ok(file) => file,
                        Err(e)
                            if e.kind() == std::io::ErrorKind::NotFound
                                && checkpoint.cleanup_pending.contains(&segment.partial) =>
                        {
                            continue
                        }
                        Err(_) => return Err(JobError::new(
                            ErrorKind::Cleanup,
                            "Cannot verify a range file for cleanup; saved bytes were preserved.",
                        )
                        .retained(segment.partial.clone())),
                    };
                delete_owned(&file, &segment.partial).map_err(|_| {
                    JobError::new(ErrorKind::Cleanup, "Cannot remove a saved range file.")
                        .retained(segment.partial.clone())
                })?;
            }
        }
        let Some(mut file) = self.file.take() else {
            return Ok(());
        };
        let _ = file.flush().await;
        let file = file.into_std().await;
        let partial = self.partial.clone();
        let result = tokio::task::spawn_blocking(move || delete_owned(&file, &partial))
            .await
            .map_err(|_| {
                JobError::new(
                    ErrorKind::Cleanup,
                    "The cleanup worker failed. The partial was retained.",
                )
                .retained(self.partial.clone())
            })?;
        match result {
            Ok(()) => {
                self.retain = true;
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.retain = true;
                Ok(())
            }
            Err(_) => {
                self.retain = true;
                Err(JobError::new(
                    ErrorKind::Cleanup,
                    "The transfer stopped, but its partial file could not be removed.",
                )
                .retained(self.partial.clone()))
            }
        }
    }
    pub async fn finalize(self) -> Result<(), JobError> {
        self.finalize_controlled(crate::transfer::TransferControl::new(
            tokio_util::sync::CancellationToken::new(),
        ))
        .await
        .map(|_| ())
    }
    pub async fn finalize_controlled(
        self,
        control: crate::transfer::TransferControl,
    ) -> Result<bool, JobError> {
        self.finalize_with_progress(control, None).await
    }
    pub(crate) async fn finalize_with_progress(
        mut self,
        control: crate::transfer::TransferControl,
        progress: Option<crate::transfer::ProgressSink>,
    ) -> Result<bool, JobError> {
        self.retain = true; // Any finalization failure retains recoverable downloaded bytes.
        self.checkpoint().await?;
        if !self
            .stage_for_publication(&control, progress.as_ref())
            .await?
        {
            if control.preserving() {
                return Ok(false);
            }
            let recovery = self.recovery.clone();
            self.abort().await?;
            if let Some(r) = recovery {
                r.clear()?;
            }
            return Ok(false);
        }
        let file = self.file.as_mut().expect("open partial");
        file.flush().await.map_err(|_| {
            JobError::new(ErrorKind::Io, "Cannot flush the completed partial file.")
                .retained(self.partial.clone())
        })?;
        file.sync_all().await.map_err(|_| {
            JobError::new(ErrorKind::Io, "Cannot sync the completed partial file.")
                .retained(self.partial.clone())
        })?;
        let completed_bytes = file
            .metadata()
            .await
            .map_err(|_| {
                JobError::new(ErrorKind::Io, "Cannot inspect the completed partial file.")
                    .retained(self.partial.clone())
            })?
            .len();
        let file = self.file.take().expect("open partial").into_std().await;
        if let Some(at) = self.creation_time {
            set_creation_time(&file, at).map_err(|_| {
                JobError::new(
                    ErrorKind::Io,
                    "Cannot set the requested server creation time; the partial was preserved.",
                )
                .retained(self.partial.clone())
            })?;
        }
        let partial = self.partial.clone();
        let destination = self.destination.clone();
        let recovery = self.recovery.clone();
        #[cfg(windows)]
        let original = self.original.take();
        let (file, result, retained_original, cleanup_failed) =
            tokio::task::spawn_blocking(move || {
                #[allow(unused_mut)]
                let mut retained_original: Option<PathBuf> = None;
                #[cfg(windows)]
                let mut backup = None;
                let result = control.commit_file(|| {
                    if let Some(r) = &recovery {
                        r.journal(
                            r.checkpoint()
                                .ok_or_else(|| std::io::Error::other("Missing final checkpoint"))?,
                            None,
                        )
                        .map_err(std::io::Error::other)?;
                    }
                    #[cfg(windows)]
                    if let Some(original) = original.as_ref() {
                        let path =
                            backup_original_journaled(original, &destination, recovery.as_ref())?;
                        publish_or_restore(original, &file, &partial, &destination, &path)
                            .map_err(|failure| {
                                retained_original = failure.retained_original;
                                failure.error
                            })?;
                        backup = Some(path);
                        return Ok(());
                    }
                    no_replace(&file, &partial, &destination)
                });
                #[allow(unused_mut)]
                let mut cleanup_failed = false;
                #[cfg(windows)]
                if let (Ok(true), Some(original), Some(backup)) =
                    (&result, original.as_ref(), backup)
                {
                    if delete_owned(original, &backup).is_err() {
                        retained_original = Some(backup);
                        cleanup_failed = true;
                    }
                }
                (file, result, retained_original, cleanup_failed)
            })
            .await
            .map_err(|_| {
                JobError::new(ErrorKind::Io, "Finalization worker failed.")
                    .retained(self.partial.clone())
            })?;
        let committed = result.map_err(|e| {
            let kind = if e.get_ref().and_then(|inner|inner.downcast_ref::<JobError>()).is_some_and(|error|error.kind==ErrorKind::Storage) { ErrorKind::Storage } else if e.kind() == std::io::ErrorKind::AlreadyExists || self.destination.exists() { ErrorKind::DestinationChanged } else { ErrorKind::Io };
            let mut error = JobError::new(kind, format!("Cannot finalize the download: {e}. The completed partial has been retained at {}.", self.partial.display())).retained(self.partial.clone());
            if let Some(original) = retained_original.as_ref() {
                error = error.original_retained(original.clone());
            }
            error
        })?;
        if cleanup_failed {
            let backup = retained_original.expect("failed original cleanup has a recovery path");
            return Err(JobError::new(ErrorKind::Cleanup, format!("The download was saved successfully at {}, but the previous file could not be removed. Its backup is retained at {}.", self.destination.display(), backup.display())).original_retained(backup));
        }
        if !committed {
            self.file = Some(File::from_std(file));
            let recovery = self.recovery.clone();
            self.abort().await?;
            if let Some(recovery) = recovery {
                recovery.clear()?;
            }
            return Ok(false);
        }
        if committed {
            if let Some(recovery) = &self.recovery {
                // Publication succeeded. A metadata write failure remains pending
                // for the actor to report/retry; it cannot undo that success.
                let _ = recovery.finish(completed_bytes);
            }
        }
        Ok(committed)
    }
}

#[cfg(not(windows))]
pub(crate) fn identify_destination(file: &std::fs::File) -> Result<DestinationIdentity, JobError> {
    let m = file
        .metadata()
        .map_err(|_| JobError::new(ErrorKind::Io, "Cannot inspect partial file."))?;
    Ok(DestinationIdentity {
        size: m.len(),
        modified: m.modified().ok(),
    })
}
fn hashes(file: &mut std::fs::File, length: u64) -> std::io::Result<(Vec<u8>, Vec<u8>)> {
    use std::io::{Read, Seek};
    file.rewind()?;
    let mut full = ring::digest::Context::new(&ring::digest::SHA256);
    let mut prefix = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buffer = [0u8; 65536];
    let mut offset = 0;
    while offset < length {
        let count = ((length - offset).min(buffer.len() as u64)) as usize;
        file.read_exact(&mut buffer[..count])?;
        full.update(&buffer[..count]);
        if offset < 65536 {
            prefix.update(&buffer[..count.min((65536 - offset) as usize)]);
        }
        offset += count as u64;
    }
    file.seek(std::io::SeekFrom::Start(length))?;
    Ok((
        prefix.finish().as_ref().to_vec(),
        full.finish().as_ref().to_vec(),
    ))
}
fn make_checkpoint(
    mut file: std::fs::File,
    partial: PathBuf,
    total: Option<u64>,
    response_url: Option<String>,
    etag: Option<String>,
) -> Result<crate::storage::Checkpoint, JobError> {
    let identity = identify_destination(&file)?;
    let durable = identity.size;
    let (prefix_hash, bytes_hash) = hashes(&mut file, durable)
        .map_err(|_| JobError::new(ErrorKind::Io, "Cannot hash checkpoint bytes."))?;
    Ok(crate::storage::Checkpoint {
        refresh_pending: false,
        creation_time: None,
        partial,
        identity,
        durable,
        prefix_hash,
        bytes_hash,
        total,
        response_url,
        etag,
        last_modified: None,
        segments: Vec::new(),
        cleanup_pending: Vec::new(),
    })
}
pub(crate) async fn reserve_segment(
    base: &Path,
    start: u64,
    end: u64,
) -> Result<(File, crate::storage::SegmentCheckpoint), JobError> {
    let parent = base
        .parent()
        .ok_or_else(|| JobError::new(ErrorKind::Io, "Invalid partial folder."))?;
    for _ in 0..128 {
        let id = NEXT_PARTIAL.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".rusty-{}-{id}.seg", std::process::id()));
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(windows)]
        {
            use windows::Win32::{
                Foundation::{GENERIC_READ, GENERIC_WRITE},
                Storage::FileSystem::{DELETE, FILE_SHARE_READ},
            };
            options
                .access_mode(GENERIC_READ.0 | GENERIC_WRITE.0 | DELETE.0)
                .share_mode(FILE_SHARE_READ.0);
        }
        match options.open(&path).await {
            Ok(file) => {
                file.sync_all()
                    .await
                    .map_err(|_| JobError::new(ErrorKind::Io, "Cannot sync a new range file."))?;
                let segment = segment_checkpoint(&file, path, start, end, 0).await?;
                return Ok((file, segment));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(JobError::new(ErrorKind::Io, "Cannot reserve a range file.")),
        }
    }
    Err(JobError::new(
        ErrorKind::Io,
        "Cannot reserve a unique range file.",
    ))
}
pub(crate) async fn segment_checkpoint(
    file: &File,
    partial: PathBuf,
    start: u64,
    end: u64,
    saved: u64,
) -> Result<crate::storage::SegmentCheckpoint, JobError> {
    file.sync_all()
        .await
        .map_err(|_| JobError::new(ErrorKind::Io, "Cannot sync range bytes."))?;
    let mut clone = file
        .try_clone()
        .await
        .map_err(|_| JobError::new(ErrorKind::Io, "Cannot inspect range bytes."))?
        .into_std()
        .await;
    tokio::task::spawn_blocking(move || {
        let identity = identify_destination(&clone)?;
        if identity.size != saved {
            return Err(JobError::new(ErrorKind::Io, "Range file length changed."));
        }
        let (prefix_hash, bytes_hash) = hashes(&mut clone, saved)
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot hash range bytes."))?;
        Ok(crate::storage::SegmentCheckpoint {
            start,
            end,
            saved,
            partial,
            identity,
            prefix_hash,
            bytes_hash,
        })
    })
    .await
    .map_err(|_| JobError::new(ErrorKind::Io, "Range checkpoint worker failed."))?
}
pub(crate) fn verify_segment(
    segment: &crate::storage::SegmentCheckpoint,
    ignore_modified: bool,
) -> std::io::Result<std::fs::File> {
    let mut file = open_owned(&segment.partial)?;
    let identity = identify_destination(&file).map_err(std::io::Error::other)?;
    if !same_file(&identity, &segment.identity)
        || identity.size < segment.saved
        || (!ignore_modified
            && identity.size == segment.saved
            && identity.modified != segment.identity.modified)
    {
        return Err(std::io::Error::other("Range identity or length changed"));
    }
    let (prefix, full) = hashes(&mut file, segment.saved)?;
    if prefix != segment.prefix_hash || full != segment.bytes_hash {
        return Err(std::io::Error::other("Recorded range bytes changed"));
    }
    file.set_len(segment.saved)?;
    file.sync_all()?;
    Ok(file)
}
pub(crate) async fn remove_segment(file: File, path: PathBuf) -> Result<(), JobError> {
    let file = file.into_std().await;
    tokio::task::spawn_blocking(move || delete_owned(&file, &path))
        .await
        .map_err(|_| JobError::new(ErrorKind::Cleanup, "Range cleanup worker failed."))?
        .map_err(|_| JobError::new(ErrorKind::Cleanup, "Cannot remove a saved range file."))
}
fn open_owned(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows::Win32::{
            Foundation::{GENERIC_READ, GENERIC_WRITE},
            Storage::FileSystem::{DELETE, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ},
        };
        options
            .access_mode(GENERIC_READ.0 | GENERIC_WRITE.0 | DELETE.0)
            .share_mode(FILE_SHARE_READ.0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0);
    }
    #[cfg(not(windows))]
    {
        if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
            return Err(std::io::Error::other("Partial is a symbolic link"));
        }
    }
    options.open(path)
}
fn same_file(a: &DestinationIdentity, b: &DestinationIdentity) -> bool {
    #[cfg(windows)]
    {
        a.volume == b.volume && a.index == b.index
    }
    #[cfg(not(windows))]
    {
        a.modified == b.modified
    }
}
fn verify(file: &mut std::fs::File, c: &crate::storage::Checkpoint) -> std::io::Result<()> {
    let identity = identify_destination(file).map_err(std::io::Error::other)?;
    if !same_file(&identity, &c.identity) || identity.size < c.durable {
        return Err(std::io::Error::other(
            "Partial identity or durable length changed",
        ));
    }
    let (prefix, full) = hashes(file, c.durable)?;
    if prefix != c.prefix_hash || full != c.bytes_hash {
        return Err(std::io::Error::other("Recorded partial bytes changed"));
    }
    Ok(())
}
pub(crate) fn inspect_checkpoint(
    c: &crate::storage::Checkpoint,
    ignore_modified: bool,
) -> std::io::Result<std::fs::File> {
    let mut file = open_owned(&c.partial)?;
    verify(&mut file, c)?;
    let identity = identify_destination(&file).map_err(std::io::Error::other)?;
    if !ignore_modified && identity.size == c.durable && identity.modified != c.identity.modified {
        return Err(std::io::Error::other("Partial modification time changed"));
    }
    Ok(file)
}
fn reopen_verified(
    c: &crate::storage::Checkpoint,
    ignore_modified: bool,
) -> std::io::Result<std::fs::File> {
    let file = inspect_checkpoint(c, ignore_modified)?;
    // Only a verified file's uncheckpointed tail can be discarded.
    file.set_len(c.durable)?;
    file.sync_all()?;
    Ok(file)
}
pub(crate) fn discard_checkpoint(recovery: &crate::storage::Recovery) -> Result<(), JobError> {
    if let Some(entry) = recovery
        .store
        .snapshot()
        .entries
        .get(&recovery.entry)
        .cloned()
    {
        recover_staging(&entry, recovery)?;
    }
    let mut preserved = None;
    if let Some(c) = recovery.checkpoint() {
        for segment in &c.segments {
            match verify_segment(segment, true) {
                Ok(file) => delete_owned(&file, &segment.partial).map_err(|_| {
                    JobError::new(
                        ErrorKind::Cleanup,
                        "Cannot remove the verified old range file.",
                    )
                    .retained(segment.partial.clone())
                })?,
                Err(e)
                    if e.kind() == std::io::ErrorKind::NotFound
                        && c.cleanup_pending.contains(&segment.partial) => {}
                Err(_) => {
                    recovery
                        .store
                        .update(|s| {
                            let entry = s
                                .entries
                                .get_mut(&recovery.entry)
                                .ok_or("Recovery entry is missing.")?;
                            if !entry.retained_files.contains(&segment.partial) {
                                entry.retained_files.push(segment.partial.clone());
                            }
                            Ok(())
                        })
                        .map_err(crate::storage::storage_error)?;
                    return Err(JobError::new(
                        ErrorKind::Cleanup,
                        "A saved range changed; its path was preserved for recovery.",
                    )
                    .retained(segment.partial.clone()));
                }
            }
        }
        match open_owned(&c.partial) {
            Ok(mut file) => {
                if verify(&mut file, &c).is_ok() {
                    delete_owned(&file, &c.partial).map_err(|_| {
                        JobError::new(
                            ErrorKind::Cleanup,
                            "Cannot remove the verified old partial.",
                        )
                        .retained(c.partial)
                    })?;
                } else {
                    preserved = Some(c.partial);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(JobError::new(
                    ErrorKind::Io,
                    "Cannot inspect the partial for a confirmed restart.",
                )
                .retained(c.partial))
            }
        }
    }
    recovery
        .store
        .update(|s| {
            let e = s
                .entries
                .get_mut(&recovery.entry)
                .ok_or("Recovery entry is missing.")?;
            if let Some(path) = preserved {
                if !e.retained_files.contains(&path) {
                    e.retained_files.push(path);
                }
            }
            e.checkpoint = None;
            e.journal = None;
            Ok(())
        })
        .map_err(crate::storage::storage_error)
}
pub(crate) fn recover_finalization(
    entry: &crate::storage::StoredEntry,
    recovery: &crate::storage::Recovery,
) -> Result<bool, JobError> {
    let Some(journal) = &entry.journal else {
        return Ok(entry.completed);
    };
    let ambiguous = || {
        let mut e = JobError::new(
            ErrorKind::Cleanup,
            "Interrupted finalization requires recovery. All ambiguous files were preserved.",
        )
        .retained(journal.checkpoint.partial.clone());
        if let Some((path, _)) = &journal.original {
            e = e.original_retained(path.clone());
        }
        e
    };
    if let Ok(mut destination) = open_owned(&entry.destination) {
        if verify(&mut destination, &journal.checkpoint).is_ok()
            && destination
                .metadata()
                .is_ok_and(|m| m.len() == journal.checkpoint.durable)
        {
            #[cfg(windows)]
            if let Some((path, identity)) = &journal.original {
                if path.exists() {
                    let original = guard_destination(path, identity).map_err(|_| ambiguous())?;
                    delete_owned(&original, path).map_err(|_| ambiguous())?;
                }
            }
            let _ = recovery.finish(journal.checkpoint.durable);
            return Ok(true);
        }
    }
    #[cfg(windows)]
    if let Some((path, identity)) = &journal.original {
        if path.exists() {
            if entry.destination.exists() {
                return Err(ambiguous());
            }
            let original = guard_destination(path, identity).map_err(|_| ambiguous())?;
            no_replace(&original, path, &entry.destination).map_err(|_| ambiguous())?;
        } else if inspect_destination(&entry.destination)
            .ok()
            .flatten()
            .as_ref()
            != Some(identity)
        {
            return Err(ambiguous());
        }
    }
    let mut partial = open_owned(&journal.checkpoint.partial).map_err(|_| ambiguous())?;
    verify(&mut partial, &journal.checkpoint).map_err(|_| ambiguous())?;
    recovery
        .store
        .update(|s| {
            s.entries
                .get_mut(&entry.id)
                .ok_or("Recovery entry is missing.")?
                .journal = None;
            Ok(())
        })
        .map_err(crate::storage::storage_error)?;
    Ok(false)
}

#[cfg(all(windows, test))]
fn backup_original(original: &std::fs::File, destination: &Path) -> std::io::Result<PathBuf> {
    backup_original_journaled(original, destination, None)
}
#[cfg(windows)]
fn backup_original_journaled(
    original: &std::fs::File,
    destination: &Path,
    recovery: Option<&crate::storage::Recovery>,
) -> std::io::Result<PathBuf> {
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    for _ in 0..128 {
        let id = NEXT_PARTIAL.fetch_add(1, Ordering::Relaxed);
        let backup = parent.join(format!(".rusty-{}-{id}.original", std::process::id()));
        if let Some(r) = recovery {
            r.journal(
                r.checkpoint()
                    .ok_or_else(|| std::io::Error::other("Missing final checkpoint"))?,
                Some((
                    backup.clone(),
                    identify_destination(original).map_err(std::io::Error::other)?,
                )),
            )
            .map_err(std::io::Error::other)?;
        }
        match no_replace(original, destination, &backup) {
            Ok(()) => return Ok(backup),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "Cannot reserve a unique backup for the approved original",
    ))
}

#[cfg(windows)]
struct ReplacementCommitError {
    error: std::io::Error,
    retained_original: Option<PathBuf>,
}

#[cfg(windows)]
fn publish_or_restore(
    original: &std::fs::File,
    partial_file: &std::fs::File,
    partial: &Path,
    destination: &Path,
    backup: &Path,
) -> Result<(), ReplacementCommitError> {
    if let Err(publish_error) = no_replace(partial_file, partial, destination) {
        // A new winner at destination must survive both commit and rollback.
        // Restore only through the guarded handle and only into an empty name.
        let (message, retained_original) = match no_replace(original, backup, destination) {
            Ok(()) => (format!("{publish_error}; the previous file was restored"), None),
            Err(restore_error) => (format!("{publish_error}; the previous file could not be restored ({restore_error}) and is retained at {}", backup.display()), Some(backup.to_owned())),
        };
        return Err(ReplacementCommitError {
            error: std::io::Error::new(publish_error.kind(), message),
            retained_original,
        });
    }
    Ok(())
}
impl Drop for OutputFile {
    fn drop(&mut self) {
        if !self.retain {
            if let Some(file) = &self.file {
                let _ = delete_owned(file, &self.partial);
            }
        }
        self.file.take();
    }
}

#[cfg(windows)]
fn no_replace(file: &std::fs::File, _source: &Path, target: &Path) -> std::io::Result<()> {
    use std::{
        mem::{offset_of, size_of},
        os::windows::{ffi::OsStrExt, io::AsRawHandle},
    };
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{FileRenameInfo, SetFileInformationByHandle, FILE_RENAME_INFO},
    };
    // Rename the original open handle, never an independently replaceable path.
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let target = std::fs::canonicalize(parent)?.join(target.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Missing destination filename",
        )
    })?);
    let target: Vec<_> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    let name_bytes = (target.len() - 1)
        .checked_mul(2)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Filename too large")
        })?;
    let size = size_of::<FILE_RENAME_INFO>() + target.len() * 2;
    let buffer_size = u32::try_from(size)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Filename too large"))?;
    // Aligned, zeroed storage sets ReplaceIfExists=false and RootDirectory=NULL.
    let mut buffer = vec![0u64; size.div_ceil(size_of::<u64>())];
    let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    unsafe {
        (*info).FileNameLength = name_bytes;
        std::ptr::copy_nonoverlapping(
            target.as_ptr(),
            buffer
                .as_mut_ptr()
                .cast::<u8>()
                .add(offset_of!(FILE_RENAME_INFO, FileName))
                .cast::<u16>(),
            target.len(),
        );
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileRenameInfo,
            info.cast(),
            buffer_size,
        )
    }
    .map_err(std::io::Error::from)
}
#[cfg(not(windows))]
fn no_replace(_file: &std::fs::File, source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::hard_link(source, target)?;
    std::fs::remove_file(source)
}

#[cfg(windows)]
fn delete_owned(
    file: &impl std::os::windows::io::AsRawHandle,
    _partial: &Path,
) -> std::io::Result<()> {
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
        },
    };
    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    unsafe {
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileDispositionInfo,
            (&disposition as *const FILE_DISPOSITION_INFO).cast(),
            std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    }
    .map_err(std::io::Error::from)
}
#[cfg(not(windows))]
fn delete_owned<T>(_file: &T, partial: &Path) -> std::io::Result<()> {
    std::fs::remove_file(partial)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn saved_range_hash_rejects_changed_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("main.part");
        let (mut file, reserved) = reserve_segment(&base, 4, 7).await.unwrap();
        file.write_all(b"data").await.unwrap();
        let recorded = segment_checkpoint(&file, reserved.partial.clone(), 4, 7, 4)
            .await
            .unwrap();
        drop(file);
        std::fs::write(&recorded.partial, b"evil").unwrap();
        assert!(verify_segment(&recorded, false).is_err());
    }
    #[tokio::test]
    async fn cleanup_failure_retains_the_actual_partial_path() {
        let dir = tempfile::tempdir().unwrap();
        let partial = dir.path().join("retained.part");
        std::fs::write(&partial, b"download").unwrap();
        // A real read-only handle lacks DELETE access: disposition must fail.
        let file = File::open(&partial).await.unwrap();
        let output = OutputFile {
            file: Some(file),
            partial: partial.clone(),
            destination: dir.path().join("file.bin"),
            retain: false,
            recovery: None,
            total: None,
            response_url: None,
            etag: None,
            refresh_pending: false,
            creation_time: None,
            original: None,
        };
        let error = output.abort().await.unwrap_err();
        assert_eq!(error.kind, ErrorKind::Cleanup);
        assert_eq!(error.retained_partial.as_ref(), Some(&partial));
        assert_eq!(std::fs::read(&partial).unwrap(), b"download");
    }
    fn substituted_partial(dir: &Path) -> (OutputFile, PathBuf) {
        use std::os::windows::fs::OpenOptionsExt;
        let partial = dir.join("original.part");
        let moved = dir.join("moved.part");
        // Deliberately relaxed sharing models a source renamed by another process.
        // Production denies that rename too; handle operations remain safe itself.
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .access_mode(0xC0010000)
            .share_mode(7)
            .open(&partial)
            .unwrap();
        std::io::Write::write_all(&mut file, b"owned bytes").unwrap();
        std::fs::rename(&partial, &moved).unwrap();
        std::fs::write(&partial, b"unrelated bytes").unwrap();
        (
            OutputFile {
                file: Some(File::from_std(file)),
                partial,
                destination: dir.join("file.bin"),
                retain: false,
                recovery: None,
                total: None,
                response_url: None,
                etag: None,
                refresh_pending: false,
                creation_time: None,
                original: None,
            },
            moved,
        )
    }
    #[tokio::test]
    async fn abort_never_deletes_a_substitute_at_the_old_partial_name() {
        let dir = tempfile::tempdir().unwrap();
        let (output, moved) = substituted_partial(dir.path());
        let old = output.partial.clone();
        output.abort().await.unwrap();
        assert_eq!(std::fs::read(old).unwrap(), b"unrelated bytes");
        assert!(!moved.exists());
    }
    #[tokio::test]
    async fn restart_truncates_only_the_owned_handle_and_rewinds_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        let (mut output, moved) = substituted_partial(dir.path());
        let old = output.partial.clone();
        let destination = output.destination.clone();
        output.restart().await.unwrap();
        output.write(b"new").await.unwrap();
        output.finalize().await.unwrap();
        assert_eq!(std::fs::read(old).unwrap(), b"unrelated bytes");
        assert_eq!(std::fs::read(destination).unwrap(), b"new");
        assert!(!moved.exists());
    }
    #[tokio::test]
    async fn finalize_moves_the_owned_handle_and_never_a_path_substitute() {
        let dir = tempfile::tempdir().unwrap();
        let (output, moved) = substituted_partial(dir.path());
        let old = output.partial.clone();
        let destination = output.destination.clone();
        output.finalize().await.unwrap();
        assert_eq!(std::fs::read(old).unwrap(), b"unrelated bytes");
        assert_eq!(std::fs::read(destination).unwrap(), b"owned bytes");
        assert!(!moved.exists());
    }
    #[tokio::test]
    async fn sync_failure_retains_partial_and_never_commits() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("file.bin");
        std::fs::write(&target, b"original").unwrap();
        let identity = inspect_destination(&target).unwrap().unwrap();
        let original = guard_destination(&target, &identity).unwrap();
        let partial = dir.path().join("read-only.part");
        std::fs::write(&partial, b"download").unwrap();
        // FlushFileBuffers requires a handle with write access. Supplying a
        // read-only handle exercises the real OS failure branch without a full disk.
        let file = tokio::fs::File::open(&partial).await.unwrap();
        let output = OutputFile {
            file: Some(file),
            partial: partial.clone(),
            destination: target.clone(),
            retain: false,
            recovery: None,
            total: None,
            response_url: None,
            etag: None,
            refresh_pending: false,
            creation_time: None,
            original: Some(original),
        };
        let error = output.finalize().await.unwrap_err();
        assert_eq!(error.retained_partial.as_ref(), Some(&partial));
        assert_eq!(std::fs::read(&target).unwrap(), b"original");
        assert_eq!(std::fs::read(&partial).unwrap(), b"download");
    }

    #[test]
    fn replacement_failure_restores_original_without_publishing_partial() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("file.bin");
        let partial = dir.path().join("download.part");
        std::fs::write(&destination, b"original").unwrap();
        std::fs::write(&partial, b"download").unwrap();
        let identity = inspect_destination(&destination).unwrap().unwrap();
        let original = guard_destination(&destination, &identity).unwrap();
        let backup = backup_original(&original, &destination).unwrap();
        // This real handle cannot rename: it deliberately has no DELETE access.
        let read_only_partial = std::fs::File::open(&partial).unwrap();
        let failure = publish_or_restore(
            &original,
            &read_only_partial,
            &partial,
            &destination,
            &backup,
        )
        .err()
        .expect("publishing a read-only handle must fail");
        assert!(failure.retained_original.is_none());
        assert_eq!(std::fs::read(&destination).unwrap(), b"original");
        assert_eq!(std::fs::read(&partial).unwrap(), b"download");
        assert!(!backup.exists());
    }

    #[tokio::test]
    async fn replacement_race_preserves_winner_backup_and_completed_partial() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("file.bin");
        std::fs::write(&destination, b"original").unwrap();
        let identity = inspect_destination(&destination).unwrap().unwrap();
        let mut output = OutputFile::prepare(&destination, &ReplacementPolicy::Approved(identity))
            .await
            .unwrap();
        output.write(b"download").await.unwrap();
        let mut file = output.file.take().unwrap();
        file.flush().await.unwrap();
        file.sync_all().await.unwrap();
        let file = file.into_std().await;
        let original = output.original.take().unwrap();
        let backup = backup_original(&original, &destination).unwrap();
        // Deterministically occupy the brief gap between the two guarded renames.
        std::fs::write(&destination, b"race winner").unwrap();
        let failure = publish_or_restore(&original, &file, &output.partial, &destination, &backup)
            .err()
            .expect("a new destination must prevent commit and rollback");
        output.retain = true;
        assert_eq!(failure.retained_original.as_ref(), Some(&backup));
        assert!(failure
            .error
            .to_string()
            .contains(&backup.display().to_string()));
        assert_eq!(std::fs::read(&destination).unwrap(), b"race winner");
        assert_eq!(std::fs::read(&backup).unwrap(), b"original");
        assert_eq!(std::fs::read(&output.partial).unwrap(), b"download");
    }
}

#[path = "output_staging.rs"]
mod staging;
pub(crate) use staging::recover_staging;
use staging::set_creation_time;

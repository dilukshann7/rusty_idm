use crate::{DestinationIdentity, ErrorKind, JobError, ReplacementPolicy};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncSeekExt, AsyncWriteExt},
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
fn identify_destination(file: &std::fs::File) -> Result<DestinationIdentity, JobError> {
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
    #[cfg(windows)]
    original: Option<std::fs::File>,
}
impl OutputFile {
    pub async fn prepare(
        destination: &Path,
        replacement: &ReplacementPolicy,
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
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        for _ in 0..128 {
            // Fixed short partial names also work beside very long final filenames.
            let id = NEXT_PARTIAL.fetch_add(1, Ordering::Relaxed);
            let partial = parent.join(format!(".rusty-{}-{id}.part", std::process::id()));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
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
                Ok(file) => return Ok(Self { file: Some(file), partial, destination: destination.to_owned(), retain: false, #[cfg(windows)] original }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(JobError::new(ErrorKind::Io, "Cannot reserve a partial file. Check the save folder, permissions and available space.")),
            }
        }
        Err(JobError::new(
            ErrorKind::Io,
            "Cannot reserve a unique partial filename.",
        ))
    }
    pub fn partial_path(&self) -> &Path {
        &self.partial
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
        mut self,
        control: crate::transfer::TransferControl,
    ) -> Result<bool, JobError> {
        self.retain = true; // Any finalization failure retains recoverable downloaded bytes.
        let file = self.file.as_mut().expect("open partial");
        file.flush().await.map_err(|_| {
            JobError::new(ErrorKind::Io, "Cannot flush the completed partial file.")
                .retained(self.partial.clone())
        })?;
        file.sync_all().await.map_err(|_| {
            JobError::new(ErrorKind::Io, "Cannot sync the completed partial file.")
                .retained(self.partial.clone())
        })?;
        let file = self.file.take().expect("open partial").into_std().await;
        let partial = self.partial.clone();
        let destination = self.destination.clone();
        #[cfg(windows)]
        let original = self.original.take();
        let (file, result, retained_original, cleanup_failed) =
            tokio::task::spawn_blocking(move || {
                #[allow(unused_mut)]
                let mut retained_original: Option<PathBuf> = None;
                #[cfg(windows)]
                let mut backup = None;
                let result = control.commit_file(|| {
                    #[cfg(windows)]
                    if let Some(original) = original.as_ref() {
                        let path = backup_original(original, &destination)?;
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
            let kind = if e.kind() == std::io::ErrorKind::AlreadyExists || self.destination.exists() { ErrorKind::DestinationChanged } else { ErrorKind::Io };
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
            self.abort().await?;
        }
        Ok(committed)
    }
}

#[cfg(windows)]
fn backup_original(original: &std::fs::File, destination: &Path) -> std::io::Result<PathBuf> {
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    for _ in 0..128 {
        let id = NEXT_PARTIAL.fetch_add(1, Ordering::Relaxed);
        let backup = parent.join(format!(".rusty-{}-{id}.original", std::process::id()));
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

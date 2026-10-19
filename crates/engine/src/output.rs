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
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::{
            Foundation::HANDLE,
            Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
        };
        let file = std::fs::File::open(path).map_err(|_| {
            JobError::new(
                ErrorKind::Io,
                "Cannot open the existing destination for inspection.",
            )
        })?;
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info) }.map_err(
            |_| JobError::new(ErrorKind::Io, "Cannot identify the existing destination."),
        )?;
        Ok(Some(DestinationIdentity {
            size: ((info.nFileSizeHigh as u64) << 32) | info.nFileSizeLow as u64,
            modified: metadata.modified().ok(),
            volume: info.dwVolumeSerialNumber,
            index: ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
        }))
    }
    #[cfg(not(windows))]
    Ok(Some(DestinationIdentity {
        size: metadata.len(),
        modified: metadata.modified().ok(),
    }))
}

/// Owns only its uniquely created partial. Completed files are never removed by Drop.
pub struct OutputFile {
    file: Option<File>,
    partial: PathBuf,
    destination: PathBuf,
    retain: bool,
}
impl OutputFile {
    pub async fn prepare(
        destination: &Path,
        replacement: &ReplacementPolicy,
    ) -> Result<Self, JobError> {
        if matches!(replacement, ReplacementPolicy::Approved(_)) {
            return Err(JobError::new(ErrorKind::DestinationConflict, "Replacement is unavailable until its safe native transaction is verified. Choose a new filename."));
        }
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
                Ok(file) => return Ok(Self { file: Some(file), partial, destination: destination.to_owned(), retain: false }),
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
        let (file, result) = tokio::task::spawn_blocking(move || {
            let result = control.commit_file(|| no_replace(&file, &partial, &destination));
            (file, result)
        })
        .await
        .map_err(|_| {
            JobError::new(ErrorKind::Io, "Finalization worker failed.")
                .retained(self.partial.clone())
        })?;
        let committed = result.map_err(|e| {
            let kind = if e.kind() == std::io::ErrorKind::AlreadyExists || self.destination.exists() { ErrorKind::DestinationChanged } else { ErrorKind::Io };
            JobError::new(kind, "Cannot finalize the download. The destination may have appeared or changed; choose another name. The completed partial has been retained.").retained(self.partial.clone())
        })?;
        if !committed {
            self.file = Some(File::from_std(file));
            self.abort().await?;
        }
        Ok(committed)
    }
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
        };
        let error = output.finalize().await.unwrap_err();
        assert_eq!(error.retained_partial.as_ref(), Some(&partial));
        assert!(!target.exists());
        assert_eq!(std::fs::read(&partial).unwrap(), b"download");
    }
}

//! Journaled publication staging; a completed source is kept until the copy is durable.
use super::*;
use tokio::io::AsyncReadExt;
impl OutputFile {
    pub(crate) fn server_creation_time(&mut self, value: Option<&str>) {
        if let Some(at) = value.and_then(http_time) {
            self.creation_time = Some(at);
        }
    }
    pub(super) async fn stage_for_publication(
        &mut self,
        control: &crate::transfer::TransferControl,
        progress: Option<&crate::transfer::ProgressSink>,
    ) -> Result<bool, JobError> {
        let parent = self.destination.parent().unwrap_or(Path::new("."));
        if self.partial.parent() == Some(parent) {
            return Ok(true);
        }
        let recovery = self.recovery.clone().ok_or_else(|| {
            JobError::new(
                ErrorKind::Storage,
                "Temporary-directory publication requires a durable recovery record.",
            )
        })?;
        let source = recovery.checkpoint().ok_or_else(|| {
            JobError::new(ErrorKind::Storage, "Missing publication source checkpoint.")
        })?;
        let mut reserved = None;
        for _ in 0..128 {
            let id = NEXT_PARTIAL.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(".rusty-final-{}-{id}.part", std::process::id()));
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
                    reserved = Some((path, file));
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => {
                    return Err(JobError::new(
                        ErrorKind::Io,
                        "Cannot reserve the final-directory staging file.",
                    )
                    .retained(self.partial.clone()))
                }
            }
        }
        let (path, mut copy) = reserved.ok_or_else(|| {
            JobError::new(
                ErrorKind::Io,
                "Cannot reserve a unique final-directory staging file.",
            )
        })?;
        copy.sync_all()
            .await
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot sync publication staging."))?;
        let raw = copy
            .try_clone()
            .await
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot inspect staging."))?
            .into_std()
            .await;
        let initial = make_checkpoint(
            raw,
            path.clone(),
            Some(source.durable),
            source.response_url.clone(),
            source.etag.clone(),
        )?;
        if let Err(error) = recovery.store.update(|s| {
            s.entries
                .get_mut(&recovery.entry)
                .ok_or("Recovery record was removed.")?
                .staging = Some(initial);
            Ok(())
        }) {
            let raw = copy.into_std().await;
            let _ = delete_owned(&raw, &path);
            return Err(crate::storage::storage_error(error).retained(self.partial.clone()));
        }
        self.file
            .as_mut()
            .expect("source partial")
            .seek(std::io::SeekFrom::Start(0))
            .await
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot rewind publication source."))?;
        let mut buffer = vec![0; 65536];
        let mut copied = 0u64;
        while copied < source.durable {
            if control.token().is_cancelled() {
                let raw = copy.into_std().await;
                delete_owned(&raw, &path).map_err(|_| {
                    JobError::new(ErrorKind::Cleanup, "Cannot clean publication staging.")
                        .retained(path.clone())
                })?;
                recovery
                    .store
                    .update(|s| {
                        s.entries
                            .get_mut(&recovery.entry)
                            .ok_or("Recovery record was removed.")?
                            .staging = None;
                        Ok(())
                    })
                    .map_err(crate::storage::storage_error)?;
                return Ok(false);
            }
            if control.is_paused() {
                self.checkpoint().await?;
                if let Some(progress) = progress {
                    progress(crate::transfer::TransferProgress {
                        received: source.durable,
                        total: Some(source.durable),
                        resume_capable: true,
                        paused: true,
                        details: None,
                    });
                }
                if !control.wait_until_resumed_output(self).await? {
                    continue;
                }
                if let Some(progress) = progress {
                    progress(crate::transfer::TransferProgress {
                        received: source.durable,
                        total: Some(source.durable),
                        resume_capable: true,
                        paused: false,
                        details: None,
                    });
                }
                self.file
                    .as_mut()
                    .expect("source")
                    .seek(std::io::SeekFrom::Start(copied))
                    .await
                    .map_err(|_| {
                        JobError::new(
                            ErrorKind::Io,
                            "Cannot restore the publication copy position.",
                        )
                    })?;
            }
            let size = (source.durable - copied).min(buffer.len() as u64) as usize;
            self.file
                .as_mut()
                .expect("source")
                .read_exact(&mut buffer[..size])
                .await
                .map_err(|_| {
                    JobError::new(ErrorKind::Io, "Cannot read the completed temporary file.")
                })?;
            copy.write_all(&buffer[..size]).await.map_err(|_| {
                JobError::new(
                    ErrorKind::Io,
                    "Cannot copy to the final folder. The source was preserved.",
                )
            })?;
            copied += size as u64;
        }
        copy.flush()
            .await
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot flush publication staging."))?;
        copy.sync_all()
            .await
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot sync publication staging."))?;
        let raw = copy
            .try_clone()
            .await
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot inspect completed staging."))?
            .into_std()
            .await;
        let mut complete = make_checkpoint(
            raw,
            path.clone(),
            source.total,
            source.response_url.clone(),
            source.etag.clone(),
        )?;
        complete.creation_time = self.creation_time;
        if complete.durable != source.durable || complete.bytes_hash != source.bytes_hash {
            return Err(JobError::new(
                ErrorKind::Io,
                "Publication copy verification failed; both files were preserved.",
            )
            .retained(path));
        }
        recovery
            .store
            .update(|s| {
                let e = s
                    .entries
                    .get_mut(&recovery.entry)
                    .ok_or("Recovery record was removed.")?;
                e.checkpoint = Some(complete);
                e.staging = None;
                if !e.retained_files.contains(&source.partial) {
                    e.retained_files.push(source.partial.clone());
                }
                Ok(())
            })
            .map_err(crate::storage::storage_error)?;
        let original = self.file.take().expect("source").into_std().await;
        self.file = Some(copy);
        self.partial = path;
        delete_owned(&original,&source.partial).map_err(|_|JobError::new(ErrorKind::Cleanup,"The final-directory copy is safe, but the original temporary copy could not be removed.").retained(source.partial.clone()))?;
        drop(original);
        recovery
            .store
            .update(|s| {
                s.entries
                    .get_mut(&recovery.entry)
                    .ok_or("Recovery record was removed.")?
                    .retained_files
                    .retain(|p| p != &source.partial);
                Ok(())
            })
            .map_err(crate::storage::storage_error)?;
        Ok(true)
    }
}
pub(crate) fn recover_staging(
    entry: &crate::storage::StoredEntry,
    recovery: &crate::storage::Recovery,
) -> Result<(), JobError> {
    let Some(copy) = &entry.staging else {
        return Ok(());
    };
    let ambiguous = || {
        JobError::new(
            ErrorKind::Cleanup,
            "Interrupted temporary-file copy needs recovery; ambiguous files were preserved.",
        )
        .retained(copy.partial.clone())
    };
    if entry.journal.is_some() {
        return Err(ambiguous());
    }
    let source = entry.checkpoint.as_ref().ok_or_else(ambiguous)?;
    let mut file = open_owned(&source.partial).map_err(|_| ambiguous())?;
    verify(&mut file, source).map_err(|_| ambiguous())?;
    match open_owned(&copy.partial) {
        Ok(mut file) => {
            verify(&mut file, copy).map_err(|_| ambiguous())?;
            delete_owned(&file, &copy.partial).map_err(|_| ambiguous())?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(ambiguous()),
    }
    recovery
        .store
        .update(|s| {
            s.entries
                .get_mut(&recovery.entry)
                .ok_or("Recovery record was removed.")?
                .staging = None;
            Ok(())
        })
        .map_err(crate::storage::storage_error)
}
fn http_time(value: &str) -> Option<std::time::SystemTime> {
    let fields: Vec<_> = value.split_whitespace().collect();
    if fields.len() != 6 || !fields[0].ends_with(',') || fields[5] != "GMT" {
        return None;
    }
    let day = fields[1].parse::<u8>().ok()?;
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .iter()
    .position(|m| *m == fields[2])? as u8
        + 1;
    let year = fields[3].parse::<u16>().ok()?;
    let date = crate::Date { year, month, day };
    if !date.valid() {
        return None;
    }
    let time: Vec<u64> = fields[4]
        .split(':')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if time.len() != 3 || time[0] >= 24 || time[1] >= 60 || time[2] >= 60 {
        return None;
    }
    let before = |y: u16| {
        let y = y as i64 - 1;
        y * 365 + y / 4 - y / 100 + y / 400
    };
    let mut days = before(year) - before(1970);
    let mut cursor = crate::Date {
        year,
        month: 1,
        day: 1,
    };
    while cursor != date {
        cursor = cursor.next()?;
        days += 1;
    }
    let seconds = days * 86400 + (time[0] * 3600 + time[1] * 60 + time[2]) as i64;
    if seconds >= 0 {
        std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(seconds as u64))
    } else {
        std::time::UNIX_EPOCH.checked_sub(std::time::Duration::from_secs((-seconds) as u64))
    }
}
#[cfg(windows)]
pub(super) fn set_creation_time(
    file: &std::fs::File,
    at: std::time::SystemTime,
) -> std::io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::{FILETIME, HANDLE},
        Storage::FileSystem::SetFileTime,
    };
    let epoch = std::time::UNIX_EPOCH
        .checked_sub(std::time::Duration::from_secs(11_644_473_600))
        .ok_or_else(|| std::io::Error::other("Invalid time epoch"))?;
    let ticks = at
        .duration_since(epoch)
        .map_err(std::io::Error::other)?
        .as_secs()
        .checked_mul(10_000_000)
        .ok_or_else(|| std::io::Error::other("Creation time overflow"))?;
    let time = FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };
    unsafe { SetFileTime(HANDLE(file.as_raw_handle()), Some(&time), None, None) }
        .map_err(std::io::Error::from)
}
#[cfg(not(windows))]
pub(super) fn set_creation_time(
    _: &std::fs::File,
    _: std::time::SystemTime,
) -> std::io::Result<()> {
    Ok(())
}

//! A module for io related Utilitys
use log::{debug, trace};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncSeekExt;
use tokio::io::AsyncWriteExt;

use std::io::Error as IoError;
use std::io::ErrorKind;

use crate::extensions::iter::IteratorExt;

/// An Error that can happen, when moving a File
#[derive(Debug, Error)]
pub enum MoveError {
    /// The file to move was not found
    #[error("file not found")]
    FileNotFound,
    /// The target to move the file to was not found
    #[error("target folder not found")]
    TargetNotFound,
    #[error(transparent)]
    /// Any other error
    OtherIO(IoError),
}
impl From<IoError> for MoveError {
    fn from(value: IoError) -> Self {
        match value.kind() {
            // some kinds are commented out because they are unstable
            ErrorKind::NotFound /*| ErrorKind::IsADirectory*/ => Self::FileNotFound,
            // ErrorKind::NotADirectory => Self::TargetNotFound,
            _ => Self::OtherIO(value),
        }
    }
}

/// moves `file` to `dst`
/// trys to rename the file, but copys an deletes old, when on differend devices
/// `dry_run` simulates the move and prints a message
///
/// # Errors
/// - [`MoveError::FileNotFound`] when `file` doesn't exist
/// - [`MoveError::TargetNotFound`] when `dst` doesn't exist
/// - [`MoveError::OtherIO`] will relay any other error
pub async fn move_file<P1: AsRef<Path> + Send + Sync, P2: AsRef<Path> + Send + Sync>(
    file: P1,
    dst: P2,
    dry_run: bool,
) -> Result<(), (MoveError, P1, P2)> {
    inner_move_file(file.as_ref(), dst.as_ref(), dry_run)
        .await
        .map_err(|err| (err, file, dst))
}
async fn inner_move_file(file: &Path, dst: &Path, dry_run: bool) -> Result<(), MoveError> {
    if !tokio::fs::try_exists(dst).await? && tokio::fs::metadata(dst).await?.is_dir() {
        return Err(MoveError::TargetNotFound);
    }
    if !tokio::fs::try_exists(file).await? && tokio::fs::metadata(dst).await?.is_file() {
        return Err(MoveError::FileNotFound);
    }
    if dry_run {
        println!("moving {file:?} to {dst:?}");
        return Ok(());
    }

    let mut dst = dst.to_path_buf();
    dst.push(file.file_name().unwrap());
    trace!("moving {file:?} to {dst:?}");
    match tokio::fs::rename(&file, &dst).await {
        Ok(()) => Ok(()),
        Err(_err) /* TODO if err.kind() == IoErrorKind::CrossesDevices is unstable*/ => {
            debug!("couldn't just rename file, try to copy and remove old");
            tokio::fs::copy(&file, &dst).await?;
            tokio::fs::remove_file(&file).await?;
            Ok(())
        }
        // Err(err) => Err(err.into()),
    }
}

/// a Wrapper, that creates a copy of a file and removes it, when dropped
pub struct TmpFile {
    path: PathBuf,
    is_removed: bool,
}
impl TmpFile {
    const fn new(path: PathBuf) -> Self {
        Self {
            path,
            is_removed: false,
        }
    }
    /// copys the file at `orig` to `path` and return a [`TmpFile`] pointed to `path`
    ///
    /// # Errors
    /// will relay any error from [coping the file](std::fs::copy)
    pub fn new_copy(path: PathBuf, orig: impl AsRef<Path>) -> Result<Self, IoError> {
        match std::fs::metadata(&path) {
            Ok(_) => Err(IoError::new(
                ErrorKind::AlreadyExists,
                format!("there is already a file at {path:?}"),
            )),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }?;
        std::fs::copy(orig, &path)?;
        Ok(Self::new(path))
    }

    /// creates a new file at `path` and returns a [`TmpFile`] pointed to `path`
    ///
    /// # Errors
    /// - [`IoError`] with kind [`ErrorKind::AlreadyExists`] when there is a file at `path`
    /// - will relay any error from [creating the file](std::fs::File::create)
    pub fn new_empty(path: PathBuf) -> Result<Self, IoError> {
        match std::fs::metadata(&path) {
            Ok(_) => Err(IoError::new(
                ErrorKind::AlreadyExists,
                format!("there is already a file at {path:?}"),
            )),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }?;
        let _ = std::fs::File::create(&path)?;
        Ok(Self::new(path))
    }
    fn remove(&mut self) -> Result<(), IoError> {
        if !self.is_removed {
            std::fs::remove_file(&self.path)?;
            self.was_removed();
        }
        Ok(())
    }
    /// mark this file as already removed
    pub fn was_removed(&mut self) {
        self.is_removed = true;
    }
}

impl AsRef<std::path::Path> for TmpFile {
    fn as_ref(&self) -> &std::path::Path {
        &self.path
    }
}
impl Drop for TmpFile {
    fn drop(&mut self) {
        self.remove().unwrap();
    }
}

/// assumes linux style \n and an extra newline at the end
pub async fn truncate_last_lines<const N: usize>(
    file: &mut tokio::fs::File,
) -> std::io::Result<()> {
    let new_line = u8::try_from('\n').unwrap();

    let mut buf = [0u8; 64];
    let mut offset = 0;
    let mut last = crate::collections::ArrayNPM::<N, 1, Option<_>>::from_fn(|_| None);
    let mut pointer = 0;

    loop {
        match file.read(&mut buf).await? {
            0 => break,
            bytes_read => {
                for (i, _) in buf[0..bytes_read]
                    .iter()
                    .copied()
                    .lzip(offset..(offset + bytes_read))
                    .filter(|&(_, byte)| byte == new_line)
                {
                    last[pointer] = Some(i);
                    pointer = (pointer + 1) % (N + 1);
                }
                offset += bytes_read;
            }
        }
    }

    if let Some(len) = last[pointer] {
        file.set_len(len as u64 + 1).await
    } else {
        // need to leave an newline char at the end
        file.seek(std::io::SeekFrom::Start(0)).await?;
        file.set_len(0).await?;
        file.write_all(b"\n").await?;
        file.flush().await
    }
}
#[tokio::test]
async fn truncate_lines() {
    async fn helper<const N: usize>() -> String {
        let data = TmpFile::new_copy(
            PathBuf::from(format!("./res/.truncate_{N}.txt")),
            "./res/truncate.txt",
        )
        .unwrap();
        let mut file = tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .open(&data)
            .await
            .unwrap();
        truncate_last_lines::<N>(&mut file).await.unwrap();

        tokio::fs::read_to_string(&data).await.unwrap()
    }
    assert_eq!("line 1\nline 2\nline 3\n", helper::<0>().await);
    assert_eq!("line 1\nline 2\n", helper::<1>().await);
    assert_eq!("line 1\n", helper::<2>().await);
    assert_eq!("\n", helper::<3>().await);
    assert_eq!("\n", helper::<4>().await);
}

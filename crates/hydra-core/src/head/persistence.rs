use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use uuid::Uuid;

use super::HeadError;

pub(super) struct StateLock {
    path: PathBuf,
    file: File,
}

impl StateLock {
    pub(super) fn acquire(state_path: &Path) -> Result<Self, HeadError> {
        let path = state_path.with_extension("json.lock");
        let file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(HeadError::StateLockExists(path));
            }
            Err(source) => {
                return Err(HeadError::FileSystem {
                    action: "create Hydra state lock",
                    path,
                    source,
                });
            }
        };
        Ok(Self { path, file })
    }

    pub(super) fn release(self) -> Result<(), HeadError> {
        drop(self.file);
        fs::remove_file(&self.path).map_err(|source| HeadError::FileSystem {
            action: "remove Hydra state lock",
            path: self.path,
            source,
        })
    }
}

pub(super) fn replace_state_atomically(
    path: &Path,
    expected: &[u8],
    replacement: &[u8],
) -> Result<(), HeadError> {
    let current = fs::read(path).map_err(|source| HeadError::FileSystem {
        action: "re-read local Hydra state",
        path: path.to_path_buf(),
        source,
    })?;
    if current != expected {
        return Err(HeadError::ConcurrentStateChange(path.to_path_buf()));
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| HeadError::UnsafeProjectFile(path.to_path_buf()))?;
    let temporary = path.with_file_name(format!(
        ".{}.tmp-{}",
        file_name.to_string_lossy(),
        Uuid::new_v4().simple()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|source| HeadError::FileSystem {
            action: "create temporary state file",
            path: temporary.clone(),
            source,
        })?;
    if let Err(source) = file.write_all(replacement).and_then(|()| file.sync_all()) {
        drop(file);
        return Err(remove_temporary_after_error(
            &temporary,
            HeadError::FileSystem {
                action: "write temporary state file",
                path: temporary.clone(),
                source,
            },
        ));
    }
    drop(file);

    if let Err(source) = fs::rename(&temporary, path) {
        return Err(remove_temporary_after_error(
            &temporary,
            HeadError::FileSystem {
                action: "publish local Hydra state",
                path: path.to_path_buf(),
                source,
            },
        ));
    }
    sync_parent_directory(path)
        .map_err(|error| HeadError::HeadCommittedWithCleanupFailure(Box::new(error)))
}

pub(super) fn create_file_atomically(path: &Path, contents: &[u8]) -> Result<(), HeadError> {
    let file_name = path
        .file_name()
        .ok_or_else(|| HeadError::UnsafeProjectFile(path.to_path_buf()))?;
    let temporary = path.with_file_name(format!(
        ".{}.tmp-{}",
        file_name.to_string_lossy(),
        Uuid::new_v4().simple()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|source| HeadError::FileSystem {
            action: "create temporary state file",
            path: temporary.clone(),
            source,
        })?;
    if let Err(source) = file.write_all(contents).and_then(|()| file.sync_all()) {
        drop(file);
        return Err(remove_temporary_after_error(
            &temporary,
            HeadError::FileSystem {
                action: "write temporary state file",
                path: temporary.clone(),
                source,
            },
        ));
    }
    drop(file);

    if let Err(source) = fs::hard_link(&temporary, path) {
        return Err(remove_temporary_after_error(
            &temporary,
            HeadError::FileSystem {
                action: "publish new state file atomically",
                path: path.to_path_buf(),
                source,
            },
        ));
    }
    if let Err(source) = fs::remove_file(&temporary) {
        return Err(HeadError::HeadCommittedWithCleanupFailure(Box::new(
            HeadError::FileSystem {
                action: "remove temporary state link",
                path: temporary,
                source,
            },
        )));
    }
    sync_parent_directory(path)
        .map_err(|error| HeadError::HeadCommittedWithCleanupFailure(Box::new(error)))
}

fn remove_temporary_after_error(path: &Path, original: HeadError) -> HeadError {
    match fs::remove_file(path) {
        Ok(()) => original,
        Err(cleanup) => HeadError::RollbackFailed {
            original: Box::new(original),
            failures: vec![
                HeadError::FileSystem {
                    action: "remove temporary state file",
                    path: path.to_path_buf(),
                    source: cleanup,
                }
                .to_string(),
            ],
        },
    }
}

#[cfg(unix)]
fn sync_parent_directory(path: &Path) -> Result<(), HeadError> {
    let parent = path
        .parent()
        .ok_or_else(|| HeadError::UnsafeProjectFile(path.to_path_buf()))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| HeadError::FileSystem {
            action: "synchronize state directory",
            path: parent.to_path_buf(),
            source,
        })
}

#[cfg(not(unix))]
fn sync_parent_directory(_path: &Path) -> Result<(), HeadError> {
    Ok(())
}

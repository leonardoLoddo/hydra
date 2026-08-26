use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::HeadError;

pub(super) struct StateLock {
    path: PathBuf,
    file: File,
    guard: File,
}

const STATE_LOCK_VERSION: u32 = 1;
const DIRECTORY_MARKER_FILE_NAME: &str = "directory.json";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StateLockRecord {
    version: u32,
}

pub(super) enum StateLockInspection {
    Absent,
    Active(PathBuf),
    Abandoned(PathBuf),
}

enum StoredStateLock {
    Absent,
    Current,
}

impl StateLock {
    pub(super) fn acquire(state_path: &Path) -> Result<Self, HeadError> {
        let path = state_path.with_extension("json.lock");
        let mut contents = serde_json::to_vec_pretty(&StateLockRecord {
            version: STATE_LOCK_VERSION,
        })
        .map_err(HeadError::SerializeState)?;
        contents.push(b'\n');
        let Some(guard) = try_lock_state_guard(state_path, &path)? else {
            return Err(HeadError::StateLockExists(path));
        };
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
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
        if let Err(source) = file.write_all(&contents).and_then(|()| file.sync_all()) {
            drop(file);
            return Err(remove_temporary_after_error(
                &path,
                HeadError::FileSystem {
                    action: "write Hydra state lock",
                    path: path.clone(),
                    source,
                },
            ));
        }
        Ok(Self { path, file, guard })
    }

    pub(super) fn release(self) -> Result<(), HeadError> {
        drop(self.file);
        let removal = fs::remove_file(&self.path).map_err(|source| HeadError::FileSystem {
            action: "remove Hydra state lock",
            path: self.path.clone(),
            source,
        });
        finish_state_guard_operation(&self.guard, &self.path, removal)
    }
}

pub(super) fn inspect_state_lock(state_path: &Path) -> Result<StateLockInspection, HeadError> {
    let lock_path = state_path.with_extension("json.lock");
    match read_stored_state_lock(&lock_path)? {
        StoredStateLock::Absent => return Ok(StateLockInspection::Absent),
        StoredStateLock::Current => {}
    }
    match try_lock_state_guard(state_path, &lock_path)? {
        Some(guard) => {
            guard.unlock().map_err(|source| HeadError::FileSystem {
                action: "release Hydra state guard",
                path: state_guard_path(state_path),
                source,
            })?;
            Ok(StateLockInspection::Abandoned(lock_path))
        }
        None => Ok(StateLockInspection::Active(lock_path)),
    }
}

pub(super) fn remove_abandoned_state_lock(state_path: &Path) -> Result<bool, HeadError> {
    let lock_path = state_path.with_extension("json.lock");
    let Some(guard) = try_lock_state_guard(state_path, &lock_path)? else {
        return Err(HeadError::StateLockExists(lock_path));
    };
    let removal = read_stored_state_lock(&lock_path).and_then(|stored| match stored {
        StoredStateLock::Current => fs::remove_file(&lock_path)
            .map(|()| true)
            .map_err(|source| HeadError::FileSystem {
                action: "remove abandoned Hydra state lock",
                path: lock_path.clone(),
                source,
            }),
        StoredStateLock::Absent => Ok(false),
    });
    finish_state_guard_operation(&guard, state_path, removal)
}

fn read_stored_state_lock(lock_path: &Path) -> Result<StoredStateLock, HeadError> {
    let metadata = match fs::symlink_metadata(lock_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(StoredStateLock::Absent);
        }
        Err(source) => {
            return Err(HeadError::FileSystem {
                action: "inspect Hydra state lock",
                path: lock_path.to_path_buf(),
                source,
            });
        }
    };
    if !metadata.is_file() {
        return Err(HeadError::UnsafeProjectFile(lock_path.to_path_buf()));
    }
    let contents = fs::read(lock_path).map_err(|source| HeadError::FileSystem {
        action: "read Hydra state lock",
        path: lock_path.to_path_buf(),
        source,
    })?;
    let record: StateLockRecord =
        serde_json::from_slice(&contents).map_err(|source| HeadError::InvalidLocalMetadata {
            kind: "Hydra state lock",
            path: lock_path.to_path_buf(),
            source,
        })?;
    if record.version != STATE_LOCK_VERSION {
        return Err(HeadError::UnsupportedLocalMetadataVersion {
            kind: "Hydra state lock",
            version: record.version,
        });
    }
    Ok(StoredStateLock::Current)
}

fn try_lock_state_guard(state_path: &Path, lock_path: &Path) -> Result<Option<File>, HeadError> {
    let guard_path = state_guard_path(state_path);
    let guard = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&guard_path)
        .map_err(|source| HeadError::FileSystem {
            action: "open Hydra state guard",
            path: guard_path.clone(),
            source,
        })?;
    match guard.try_lock() {
        Ok(()) => Ok(Some(guard)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(source)) => Err(HeadError::FileSystem {
            action: "acquire Hydra state guard",
            path: lock_path.to_path_buf(),
            source,
        }),
    }
}

fn state_guard_path(state_path: &Path) -> PathBuf {
    state_path.with_file_name(DIRECTORY_MARKER_FILE_NAME)
}

fn finish_state_guard_operation<T>(
    guard: &File,
    state_path: &Path,
    result: Result<T, HeadError>,
) -> Result<T, HeadError> {
    let unlock = guard.unlock().map_err(|source| HeadError::FileSystem {
        action: "release Hydra state guard",
        path: state_guard_path(state_path),
        source,
    });
    match (result, unlock) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(original), Err(cleanup)) => Err(HeadError::RollbackFailed {
            original: Box::new(original),
            failures: vec![cleanup.to_string()],
        }),
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
#[allow(clippy::unnecessary_wraps)]
fn sync_parent_directory(_path: &Path) -> Result<(), HeadError> {
    Ok(())
}

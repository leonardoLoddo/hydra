use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{InitError, configuration::InitialMetadata, persistence::sync_parent_directory};

const JOURNAL_VERSION: u32 = 1;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct JournalRecord {
    version: u32,
    repository_root: PathBuf,
    heads_directory: PathBuf,
    configuration: String,
    locator: String,
    marker: String,
    inventory: String,
}

pub(super) struct InitializationJournal {
    path: PathBuf,
    file: File,
    record: JournalRecord,
}

impl InitializationJournal {
    pub(super) fn exists(path: &Path) -> Result<bool, InitError> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => Ok(true),
            Ok(_) => Err(InitError::InterruptedInitializationMismatch(
                path.to_path_buf(),
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(source) => Err(InitError::FileSystem {
                action: "inspect initialization journal",
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    pub(super) fn create(
        path: &Path,
        repository_root: &Path,
        heads_directory: &Path,
        metadata: &InitialMetadata,
    ) -> Result<Self, InitError> {
        let record = JournalRecord {
            version: JOURNAL_VERSION,
            repository_root: repository_root.to_path_buf(),
            heads_directory: heads_directory.to_path_buf(),
            configuration: bytes_as_string(&metadata.configuration)?,
            locator: bytes_as_string(&metadata.locator)?,
            marker: bytes_as_string(&metadata.marker)?,
            inventory: bytes_as_string(&metadata.inventory)?,
        };
        let mut contents =
            serde_json::to_vec_pretty(&record).map_err(InitError::SerializeConfiguration)?;
        contents.push(b'\n');
        let temporary = path.with_file_name(format!(".hydra-init.tmp-{}", Uuid::new_v4().simple()));
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|source| InitError::FileSystem {
                action: "create initialization journal",
                path: temporary.clone(),
                source,
            })?;
        file.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => InitError::InitializationInProgress(path.to_path_buf()),
            TryLockError::Error(source) => InitError::FileSystem {
                action: "lock initialization journal",
                path: temporary.clone(),
                source,
            },
        })?;
        if let Err(source) = file.write_all(&contents).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(&temporary);
            return Err(InitError::FileSystem {
                action: "write initialization journal",
                path: temporary,
                source,
            });
        }
        if let Err(source) = fs::hard_link(&temporary, path) {
            drop(file);
            let _ = fs::remove_file(&temporary);
            return if source.kind() == std::io::ErrorKind::AlreadyExists {
                Err(InitError::InitializationInProgress(path.to_path_buf()))
            } else {
                Err(InitError::FileSystem {
                    action: "publish initialization journal",
                    path: path.to_path_buf(),
                    source,
                })
            };
        }
        fs::remove_file(&temporary).map_err(|source| InitError::FileSystem {
            action: "remove initialization journal temporary link",
            path: temporary,
            source,
        })?;
        sync_parent_directory(path).map_err(|source| InitError::FileSystem {
            action: "synchronize initialization journal",
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            record,
        })
    }

    pub(super) fn acquire(
        path: &Path,
        repository_root: &Path,
        heads_directory: &Path,
    ) -> Result<Self, InitError> {
        if !Self::exists(path)? {
            return Err(InitError::InterruptedInitializationMismatch(
                path.to_path_buf(),
            ));
        }
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|source| InitError::FileSystem {
                action: "open initialization journal",
                path: path.to_path_buf(),
                source,
            })?;
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(InitError::InitializationInProgress(path.to_path_buf()));
            }
            Err(TryLockError::Error(source)) => {
                return Err(InitError::FileSystem {
                    action: "lock initialization journal",
                    path: path.to_path_buf(),
                    source,
                });
            }
        }
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|source| InitError::FileSystem {
                action: "read initialization journal",
                path: path.to_path_buf(),
                source,
            })?;
        let record: JournalRecord = serde_json::from_slice(&contents).map_err(|source| {
            InitError::InvalidLocalMetadata {
                kind: "initialization journal",
                path: path.to_path_buf(),
                source,
            }
        })?;
        if record.version != JOURNAL_VERSION {
            return Err(InitError::UnsupportedLocalMetadataVersion {
                kind: "initialization journal",
                version: record.version,
            });
        }
        if record.repository_root != repository_root || record.heads_directory != heads_directory {
            return Err(InitError::InterruptedInitializationMismatch(
                path.to_path_buf(),
            ));
        }
        Ok(Self {
            path: path.to_path_buf(),
            file,
            record,
        })
    }

    pub(super) fn metadata(&self) -> InitialMetadata {
        InitialMetadata {
            configuration: self.record.configuration.as_bytes().to_vec(),
            locator: self.record.locator.as_bytes().to_vec(),
            marker: self.record.marker.as_bytes().to_vec(),
            inventory: self.record.inventory.as_bytes().to_vec(),
        }
    }

    pub(super) fn release(self) -> Result<(), InitError> {
        fs::remove_file(&self.path).map_err(|source| InitError::FileSystem {
            action: "remove initialization journal",
            path: self.path.clone(),
            source,
        })?;
        self.file.unlock().map_err(|source| InitError::FileSystem {
            action: "unlock initialization journal",
            path: self.path,
            source,
        })
    }
}

fn bytes_as_string(bytes: &[u8]) -> Result<String, InitError> {
    String::from_utf8(bytes.to_vec())
        .map_err(|_| InitError::InvalidGitOutput("serialized initialization metadata"))
}

#[cfg(test)]
mod tests {
    use super::InitializationJournal;
    use crate::init::{InitError, configuration::InitialMetadata};

    #[test]
    fn an_active_initialization_journal_cannot_be_acquired_twice() {
        let temporary = tempfile::tempdir().expect("temporary directory should be created");
        let path = temporary.path().join("hydra-init.json");
        let repository = temporary.path().join("project");
        let heads = temporary.path().join("project.heads");
        let metadata = InitialMetadata {
            configuration: b"configuration\n".to_vec(),
            locator: b"locator\n".to_vec(),
            marker: b"marker\n".to_vec(),
            inventory: b"inventory\n".to_vec(),
        };
        let _active = InitializationJournal::create(&path, &repository, &heads, &metadata)
            .expect("journal should be created");

        let Err(error) = InitializationJournal::acquire(&path, &repository, &heads) else {
            panic!("active journal should remain exclusively locked");
        };

        assert!(matches!(error, InitError::InitializationInProgress(_)));
    }
}

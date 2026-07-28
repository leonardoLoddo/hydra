use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
};

use uuid::Uuid;

use super::{
    InitError, StorageBackend,
    artifacts::{OwnedArtifacts, rollback_owned_artifacts},
    configuration::InitialMetadata,
    storage::probe_storage,
};

pub(super) struct InitialFiles<'a> {
    pub(super) heads_directory: &'a Path,
    pub(super) heads_metadata_directory: &'a Path,
    pub(super) marker_path: &'a Path,
    pub(super) inventory_path: &'a Path,
    pub(super) state_directory: &'a Path,
    pub(super) locator_path: &'a Path,
    pub(super) configuration_path: &'a Path,
}

pub(super) fn create_initial_files(
    files: &InitialFiles<'_>,
    metadata: &InitialMetadata,
) -> Result<StorageBackend, InitError> {
    create_directory(files.heads_directory, "create Heads directory")?;

    let storage_backend = match probe_storage(files.heads_directory) {
        Ok(backend) => backend,
        Err(error) => {
            return Err(rollback_owned_artifacts(
                error,
                OwnedArtifacts {
                    files: Vec::new(),
                    directories: vec![files.heads_directory.to_path_buf()],
                },
            ));
        }
    };

    if let Err(error) = create_directory(
        files.heads_metadata_directory,
        "create Heads metadata directory",
    ) {
        return Err(rollback_owned_artifacts(
            error,
            OwnedArtifacts {
                files: Vec::new(),
                directories: vec![files.heads_directory.to_path_buf()],
            },
        ));
    }

    if let Err(error) = create_directory(files.state_directory, "create state directory") {
        return Err(rollback_owned_artifacts(
            error,
            OwnedArtifacts {
                files: Vec::new(),
                directories: vec![
                    files.heads_metadata_directory.to_path_buf(),
                    files.heads_directory.to_path_buf(),
                ],
            },
        ));
    }

    if let Err(error) = write_atomic(files.marker_path, &metadata.marker) {
        return Err(rollback_owned_artifacts(
            error,
            OwnedArtifacts {
                files: Vec::new(),
                directories: vec![
                    files.state_directory.to_path_buf(),
                    files.heads_metadata_directory.to_path_buf(),
                    files.heads_directory.to_path_buf(),
                ],
            },
        ));
    }

    if let Err(error) = write_atomic(files.inventory_path, &metadata.inventory) {
        return Err(rollback_owned_artifacts(
            error,
            OwnedArtifacts {
                files: vec![files.marker_path.to_path_buf()],
                directories: vec![
                    files.state_directory.to_path_buf(),
                    files.heads_metadata_directory.to_path_buf(),
                    files.heads_directory.to_path_buf(),
                ],
            },
        ));
    }

    if let Err(error) = write_atomic(files.locator_path, &metadata.locator) {
        return Err(rollback_owned_artifacts(
            error,
            OwnedArtifacts {
                files: vec![
                    files.inventory_path.to_path_buf(),
                    files.marker_path.to_path_buf(),
                ],
                directories: vec![
                    files.state_directory.to_path_buf(),
                    files.heads_metadata_directory.to_path_buf(),
                    files.heads_directory.to_path_buf(),
                ],
            },
        ));
    }

    if let Err(error) = write_atomic(files.configuration_path, &metadata.configuration) {
        return Err(rollback_owned_artifacts(
            error,
            OwnedArtifacts {
                files: vec![
                    files.locator_path.to_path_buf(),
                    files.inventory_path.to_path_buf(),
                    files.marker_path.to_path_buf(),
                ],
                directories: vec![
                    files.state_directory.to_path_buf(),
                    files.heads_metadata_directory.to_path_buf(),
                    files.heads_directory.to_path_buf(),
                ],
            },
        ));
    }

    Ok(storage_backend)
}

fn create_directory(path: &Path, action: &'static str) -> Result<(), InitError> {
    fs::create_dir(path).map_err(|source| InitError::FileSystem {
        action,
        path: path.to_path_buf(),
        source,
    })
}

fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), InitError> {
    let file_name = path
        .file_name()
        .ok_or_else(|| InitError::UnsupportedRepositoryPath(path.to_path_buf()))?;
    let temporary_path = path.with_file_name(format!(
        ".{}.tmp-{}",
        file_name.to_string_lossy(),
        Uuid::new_v4().simple()
    ));

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)
        .map_err(|source| InitError::FileSystem {
            action: "create temporary file",
            path: temporary_path.clone(),
            source,
        })?;
    let temporary_artifact = || OwnedArtifacts {
        files: vec![temporary_path.clone()],
        directories: Vec::new(),
    };

    if let Err(source) = file.write_all(contents).and_then(|()| file.sync_all()) {
        return Err(rollback_owned_artifacts(
            InitError::FileSystem {
                action: "write temporary file",
                path: temporary_path.clone(),
                source,
            },
            temporary_artifact(),
        ));
    }
    drop(file);

    if let Err(source) = fs::hard_link(&temporary_path, path) {
        return Err(rollback_owned_artifacts(
            InitError::FileSystem {
                action: "publish new file atomically",
                path: path.to_path_buf(),
                source,
            },
            temporary_artifact(),
        ));
    }

    if let Err(source) = fs::remove_file(&temporary_path) {
        return Err(cleanup_failed_temporary_link_removal(
            path,
            &temporary_path,
            source,
        ));
    }

    if let Err(source) = sync_parent_directory(path) {
        return Err(rollback_owned_artifacts(
            InitError::FileSystem {
                action: "synchronize parent directory",
                path: path.to_path_buf(),
                source,
            },
            OwnedArtifacts {
                files: vec![path.to_path_buf()],
                directories: Vec::new(),
            },
        ));
    }

    Ok(())
}

fn cleanup_failed_temporary_link_removal(
    published_path: &Path,
    temporary_path: &Path,
    source: io::Error,
) -> InitError {
    rollback_owned_artifacts(
        InitError::FileSystem {
            action: "remove published file temporary link",
            path: temporary_path.to_path_buf(),
            source,
        },
        OwnedArtifacts {
            files: vec![published_path.to_path_buf(), temporary_path.to_path_buf()],
            directories: Vec::new(),
        },
    )
}

#[cfg(unix)]
fn sync_parent_directory(path: &Path) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "published file has no parent directory",
        )
    })?;
    File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent_directory(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, io};

    use super::{
        InitialFiles, cleanup_failed_temporary_link_removal, create_initial_files, write_atomic,
    };
    use crate::init::{InitError, configuration::InitialMetadata};

    #[test]
    fn atomic_publication_never_replaces_an_existing_destination() {
        let temporary = tempfile::tempdir().expect("temporary directory should be created");
        let destination = temporary.path().join("state.json");
        fs::write(&destination, b"existing\n").expect("existing file should be created");

        let error = write_atomic(&destination, b"replacement\n")
            .expect_err("atomic publication must refuse to replace an existing path");

        assert!(matches!(error, InitError::FileSystem { .. }));
        assert_eq!(
            fs::read(&destination).expect("existing file should remain readable"),
            b"existing\n"
        );
        assert_eq!(
            fs::read_dir(temporary.path())
                .expect("directory should remain readable")
                .count(),
            1,
            "failed publication must remove its temporary file"
        );
    }

    #[test]
    fn publication_cleanup_reports_a_temporary_link_that_remains() {
        let temporary = tempfile::tempdir().expect("temporary directory should be created");
        let published_path = temporary.path().join("published.json");
        let temporary_path = temporary.path().join("temporary-link");
        fs::write(&published_path, b"published\n").expect("published file should be created");
        fs::create_dir(&temporary_path)
            .expect("directory simulates a temporary link that cannot be removed as a file");

        let error = cleanup_failed_temporary_link_removal(
            &published_path,
            &temporary_path,
            io::Error::new(io::ErrorKind::PermissionDenied, "simulated removal failure"),
        );

        assert!(
            !published_path.exists(),
            "failed publication must remove the final link"
        );
        match error {
            InitError::RollbackFailed {
                cleanup_failures, ..
            } => {
                assert_eq!(cleanup_failures.len(), 1);
                assert_eq!(cleanup_failures[0].path, temporary_path);
            }
            other => panic!("expected reported cleanup failure, got {other:?}"),
        }
    }

    #[test]
    fn initialization_rolls_back_all_local_metadata_when_configuration_publication_conflicts() {
        let temporary = tempfile::tempdir().expect("temporary directory should be created");
        let heads_directory = temporary.path().join("project.heads");
        let heads_metadata_directory = heads_directory.join(".hydra");
        let marker_path = heads_metadata_directory.join("directory.json");
        let inventory_path = heads_metadata_directory.join("heads.json");
        let state_directory = temporary.path().join("git-common/hydra");
        let locator_path = state_directory.join("project.json");
        fs::create_dir_all(
            state_directory
                .parent()
                .expect("state directory should have a parent"),
        )
        .expect("Git common directory should be created");
        let configuration_path = temporary.path().join(".hydra.json");
        fs::write(&configuration_path, b"preexisting\n")
            .expect("configuration conflict should be created");

        let files = InitialFiles {
            heads_directory: &heads_directory,
            heads_metadata_directory: &heads_metadata_directory,
            marker_path: &marker_path,
            inventory_path: &inventory_path,
            state_directory: &state_directory,
            locator_path: &locator_path,
            configuration_path: &configuration_path,
        };
        let metadata = InitialMetadata {
            configuration: b"configuration\n".to_vec(),
            locator: b"locator\n".to_vec(),
            marker: b"marker\n".to_vec(),
            inventory: b"inventory\n".to_vec(),
        };

        create_initial_files(&files, &metadata)
            .expect_err("configuration publication conflict should fail");

        assert_eq!(
            fs::read(&configuration_path).expect("preexisting configuration should remain"),
            b"preexisting\n"
        );
        assert!(
            !heads_directory.exists(),
            "rollback should remove the owned Heads tree"
        );
        assert!(
            !state_directory.exists(),
            "rollback should remove the owned locator directory"
        );
    }
}

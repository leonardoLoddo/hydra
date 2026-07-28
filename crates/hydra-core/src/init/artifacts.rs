use std::{fs, io, path::PathBuf};

use super::{CleanupFailure, InitError};

pub(super) struct OwnedArtifacts {
    pub(super) files: Vec<PathBuf>,
    pub(super) directories: Vec<PathBuf>,
}

pub(super) fn rollback_owned_artifacts(
    original: InitError,
    artifacts: OwnedArtifacts,
) -> InitError {
    let mut cleanup_failures = remove_owned_files(artifacts.files);
    for path in artifacts.directories {
        if let Err(source) = fs::remove_dir(&path)
            && source.kind() != io::ErrorKind::NotFound
        {
            cleanup_failures.push(CleanupFailure { path, source });
        }
    }

    if cleanup_failures.is_empty() {
        original
    } else {
        InitError::RollbackFailed {
            original: Box::new(original),
            cleanup_failures,
        }
    }
}

pub(super) fn remove_owned_files(paths: Vec<PathBuf>) -> Vec<CleanupFailure> {
    let mut cleanup_failures = Vec::new();
    for path in paths {
        if let Err(source) = fs::remove_file(&path)
            && source.kind() != io::ErrorKind::NotFound
        {
            cleanup_failures.push(CleanupFailure { path, source });
        }
    }
    cleanup_failures
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{OwnedArtifacts, rollback_owned_artifacts};
    use crate::init::InitError;

    #[test]
    fn rollback_reports_an_owned_directory_that_cannot_be_removed() {
        let temporary = tempfile::tempdir().expect("temporary directory should be created");
        let heads_directory = temporary.path().join("project.heads");
        fs::create_dir(&heads_directory).expect("Heads directory should be created");
        fs::write(heads_directory.join("unexpected"), b"preserve me")
            .expect("unexpected file should be created");

        let original = InitError::UnsafeStateDirectory(temporary.path().join("state"));
        let error = rollback_owned_artifacts(
            original,
            OwnedArtifacts {
                files: Vec::new(),
                directories: vec![heads_directory.clone()],
            },
        );

        match error {
            InitError::RollbackFailed {
                cleanup_failures, ..
            } => {
                assert_eq!(cleanup_failures.len(), 1);
                assert_eq!(cleanup_failures[0].path, heads_directory);
            }
            other => panic!("expected rollback failure, got {other:?}"),
        }
    }
}

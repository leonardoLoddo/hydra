use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
};

use uuid::Uuid;

use super::{
    InitError,
    artifacts::{OwnedArtifacts, remove_owned_files, rollback_owned_artifacts},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageBackend {
    CopyOnWrite,
    FullCopy,
}

pub(crate) fn probe_storage(destination: &Path) -> Result<StorageBackend, InitError> {
    probe_storage_with(destination, |source, target| {
        reflink_copy::reflink(source, target)
    })
}

pub(crate) fn probe_full_copy(destination: &Path) -> Result<(), InitError> {
    probe_storage_with(destination, |_, _| {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native clone intentionally bypassed for fallback verification",
        ))
    })
    .map(|_| ())
}

fn probe_storage_with(
    destination: &Path,
    clone_file: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> Result<StorageBackend, InitError> {
    const PROBE_CONTENTS: &[u8] = b"Hydra copy-on-write capability probe\n";

    let identifier = Uuid::new_v4().simple();
    let source_path = destination.join(format!(".hydra-storage-probe-{identifier}.source"));
    let target_path = destination.join(format!(".hydra-storage-probe-{identifier}.target"));
    let probe_artifacts = || OwnedArtifacts {
        files: vec![target_path.clone(), source_path.clone()],
        directories: Vec::new(),
    };

    let mut source = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&source_path)
        .map_err(|source| InitError::FileSystem {
            action: "create storage probe source",
            path: source_path.clone(),
            source,
        })?;
    if let Err(source) = source
        .write_all(PROBE_CONTENTS)
        .and_then(|()| source.sync_all())
    {
        return Err(rollback_owned_artifacts(
            InitError::FileSystem {
                action: "write storage probe source",
                path: source_path.clone(),
                source,
            },
            probe_artifacts(),
        ));
    }
    drop(source);

    let backend = if clone_file(&source_path, &target_path).is_ok() {
        StorageBackend::CopyOnWrite
    } else {
        let cleanup_failures = remove_owned_files(vec![target_path.clone()]);
        if !cleanup_failures.is_empty() {
            let mut cleanup_failures = cleanup_failures;
            cleanup_failures.extend(remove_owned_files(vec![source_path.clone()]));
            return Err(InitError::CleanupFailed {
                operation: "preparing the full-copy storage fallback",
                cleanup_failures,
            });
        }
        if let Err(error) = copy_file_exclusive(&source_path, &target_path) {
            return Err(rollback_owned_artifacts(error, probe_artifacts()));
        }
        StorageBackend::FullCopy
    };

    match fs::read(&target_path) {
        Ok(contents) if contents == PROBE_CONTENTS => {}
        Ok(_) => {
            return Err(rollback_owned_artifacts(
                InitError::InvalidStorageProbe(target_path.clone()),
                probe_artifacts(),
            ));
        }
        Err(source) => {
            return Err(rollback_owned_artifacts(
                InitError::FileSystem {
                    action: "verify storage probe",
                    path: target_path.clone(),
                    source,
                },
                probe_artifacts(),
            ));
        }
    }

    let cleanup_failures = remove_owned_files(probe_artifacts().files);
    if cleanup_failures.is_empty() {
        Ok(backend)
    } else {
        Err(InitError::CleanupFailed {
            operation: "cleaning the storage probe",
            cleanup_failures,
        })
    }
}

fn copy_file_exclusive(source_path: &Path, target_path: &Path) -> Result<(), InitError> {
    let mut source = File::open(source_path).map_err(|source| InitError::FileSystem {
        action: "open storage probe source",
        path: source_path.to_path_buf(),
        source,
    })?;
    let mut target = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target_path)
        .map_err(|source| InitError::FileSystem {
            action: "create full-copy storage probe",
            path: target_path.to_path_buf(),
            source,
        })?;
    io::copy(&mut source, &mut target)
        .and_then(|_| target.sync_all())
        .map_err(|source| InitError::FileSystem {
            action: "write full-copy storage probe",
            path: target_path.to_path_buf(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use std::{fs, io};

    use super::{StorageBackend, probe_storage_with};

    #[test]
    fn storage_probe_verifies_the_full_copy_fallback_and_removes_its_files() {
        let temporary = tempfile::tempdir().expect("temporary directory should be created");

        let backend = probe_storage_with(temporary.path(), |_, _| {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "simulated unsupported clone",
            ))
        })
        .expect("full copy fallback should be verified");

        assert_eq!(backend, StorageBackend::FullCopy);
        assert_eq!(
            fs::read_dir(temporary.path())
                .expect("probe directory should be readable")
                .count(),
            0,
            "storage probe must remove every temporary artifact"
        );
    }

    #[test]
    fn storage_probe_cleanup_error_identifies_the_leftover_path() {
        let temporary = tempfile::tempdir().expect("temporary directory should be created");

        let error = probe_storage_with(temporary.path(), |_, target| {
            fs::create_dir(target)?;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "simulated unsupported clone",
            ))
        })
        .expect_err("an unremovable probe target must fail");

        let leftover = fs::read_dir(temporary.path())
            .expect("probe directory should be readable")
            .next()
            .expect("failed cleanup should leave its target")
            .expect("leftover entry should be readable")
            .path();
        assert!(
            error.to_string().contains(&leftover.display().to_string()),
            "cleanup diagnostic should identify the exact leftover path, got: {error}"
        );

        fs::remove_dir(leftover).expect("test-owned leftover should be removed");
    }
}

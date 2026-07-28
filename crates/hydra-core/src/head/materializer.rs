use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Component, Path},
    process::{Command, Stdio},
};

#[cfg(unix)]
use std::{
    ffi::OsString,
    os::unix::{
        ffi::OsStringExt,
        fs::{PermissionsExt, symlink},
    },
};

use uuid::Uuid;

use super::{
    HeadError,
    git::{self, Repository, TrackedEntry},
};
use crate::StorageBackend;

pub(super) fn materialize_tracked_files(
    repository: &Repository,
    heads_directory: &Path,
    head_path: &Path,
    base_commit: &str,
) -> Result<StorageBackend, HeadError> {
    let mut backend = StorageBackend::CopyOnWrite;
    for entry in git::tracked_entries(repository, base_commit)? {
        validate_relative_path(&entry.path)?;
        let destination = head_path.join(&entry.path);
        match entry.mode.as_str() {
            "100644" | "100755" => {
                create_parent(&destination)?;
                if materialize_regular_file(repository, heads_directory, &entry, &destination)?
                    == StorageBackend::FullCopy
                {
                    backend = StorageBackend::FullCopy;
                }
                set_executable(&destination, entry.mode == "100755")?;
            }
            "120000" => materialize_symlink(repository, &entry, &destination)?,
            "160000" => create_directory(&destination)?,
            _ => {
                return Err(HeadError::UnsupportedTrackedEntry {
                    mode: entry.mode,
                    path: entry.path,
                });
            }
        }
    }
    Ok(backend)
}

fn materialize_regular_file(
    repository: &Repository,
    heads_directory: &Path,
    entry: &TrackedEntry,
    destination: &Path,
) -> Result<StorageBackend, HeadError> {
    let temporary_path = heads_directory.join(format!(
        ".hydra-blob-{}-{}",
        entry.object,
        Uuid::new_v4().simple()
    ));
    write_blob_to_file(repository, &entry.object, &temporary_path)?;

    let backend = match reflink_copy::reflink(&temporary_path, destination) {
        Ok(()) => Ok(StorageBackend::CopyOnWrite),
        Err(_) => copy_exclusive(&temporary_path, destination)
            .map(|()| StorageBackend::FullCopy)
            .map_err(|source| HeadError::FileSystem {
                action: "copy tracked file",
                path: destination.to_path_buf(),
                source,
            }),
    };
    let cleanup = fs::remove_file(&temporary_path).map_err(|source| HeadError::FileSystem {
        action: "remove temporary Git blob",
        path: temporary_path,
        source,
    });

    match (backend, cleanup) {
        (Ok(backend), Ok(())) => Ok(backend),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(original), Err(cleanup)) => Err(HeadError::RollbackFailed {
            original: Box::new(original),
            failures: vec![cleanup.to_string()],
        }),
    }
}

fn write_blob_to_file(
    repository: &Repository,
    object: &str,
    destination: &Path,
) -> Result<(), HeadError> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|source| HeadError::FileSystem {
            action: "create temporary Git blob",
            path: destination.to_path_buf(),
            source,
        })?;
    let output = match Command::new("git")
        .arg("-C")
        .arg(&repository.root)
        .args(["cat-file", "blob", object])
        .stdout(Stdio::from(file))
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => output,
        Err(source) => {
            return Err(remove_temporary_blob_after_error(
                destination,
                HeadError::GitUnavailable(source),
            ));
        }
    };
    if !output.status.success() {
        return Err(remove_temporary_blob_after_error(
            destination,
            HeadError::GitCommandFailed {
                operation: "reading a tracked Git blob",
                status: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            },
        ));
    }
    if let Err(source) = File::open(destination).and_then(|file| file.sync_all()) {
        return Err(remove_temporary_blob_after_error(
            destination,
            HeadError::FileSystem {
                action: "synchronize temporary Git blob",
                path: destination.to_path_buf(),
                source,
            },
        ));
    }
    Ok(())
}

fn remove_temporary_blob_after_error(path: &Path, original: HeadError) -> HeadError {
    match fs::remove_file(path) {
        Ok(()) => original,
        Err(cleanup) => HeadError::RollbackFailed {
            original: Box::new(original),
            failures: vec![
                HeadError::FileSystem {
                    action: "remove temporary Git blob",
                    path: path.to_path_buf(),
                    source: cleanup,
                }
                .to_string(),
            ],
        },
    }
}

fn copy_exclusive(source: &Path, destination: &Path) -> io::Result<()> {
    let mut source = File::open(source)?;
    let mut destination = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    io::copy(&mut source, &mut destination)?;
    destination.flush()?;
    destination.sync_all()
}

#[cfg(unix)]
fn materialize_symlink(
    repository: &Repository,
    entry: &TrackedEntry,
    destination: &Path,
) -> Result<(), HeadError> {
    create_parent(destination)?;
    let output = Command::new("git")
        .arg("-C")
        .arg(&repository.root)
        .args(["cat-file", "blob", &entry.object])
        .output()
        .map_err(HeadError::GitUnavailable)?;
    if !output.status.success() {
        return Err(HeadError::GitCommandFailed {
            operation: "reading a tracked symlink",
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    symlink(OsString::from_vec(output.stdout), destination).map_err(|source| {
        HeadError::FileSystem {
            action: "create tracked symlink",
            path: destination.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn materialize_symlink(
    _repository: &Repository,
    entry: &TrackedEntry,
    _destination: &Path,
) -> Result<(), HeadError> {
    Err(HeadError::UnsupportedTrackedEntry {
        mode: entry.mode.clone(),
        path: entry.path.clone(),
    })
}

fn validate_relative_path(path: &Path) -> Result<(), HeadError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(HeadError::UnsupportedTrackedEntry {
            mode: "unsafe-path".to_owned(),
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn create_parent(path: &Path) -> Result<(), HeadError> {
    let parent = path.parent().ok_or_else(|| HeadError::FileSystem {
        action: "resolve tracked file parent",
        path: path.to_path_buf(),
        source: io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"),
    })?;
    fs::create_dir_all(parent).map_err(|source| HeadError::FileSystem {
        action: "create tracked file parent",
        path: parent.to_path_buf(),
        source,
    })
}

fn create_directory(path: &Path) -> Result<(), HeadError> {
    fs::create_dir_all(path).map_err(|source| HeadError::FileSystem {
        action: "create submodule directory",
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(unix)]
fn set_executable(path: &Path, executable: bool) -> Result<(), HeadError> {
    let mode = if executable { 0o755 } else { 0o644 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|source| {
        HeadError::FileSystem {
            action: "set tracked file permissions",
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn set_executable(_path: &Path, _executable: bool) -> Result<(), HeadError> {
    Ok(())
}

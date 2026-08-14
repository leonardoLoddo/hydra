mod blob_batch;

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Component, Path},
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

use self::blob_batch::GitBlobBatch;
use super::{
    HeadError,
    git::{Repository, TrackedEntry},
};
use crate::StorageBackend;

pub(super) fn materialize_tracked_files(
    repository: &Repository,
    heads_directory: &Path,
    head_path: &Path,
    tracked_entries: &[TrackedEntry],
    reuse_tracked_sources: bool,
    force_full_copy: bool,
) -> Result<StorageBackend, HeadError> {
    if tracked_entries.is_empty() {
        return Ok(StorageBackend::CopyOnWrite);
    }
    let mut backend = StorageBackend::CopyOnWrite;
    let reusable_source_root = if reuse_tracked_sources {
        Some(
            fs::canonicalize(&repository.root).map_err(|source| HeadError::FileSystem {
                action: "resolve reusable tracked source root",
                path: repository.root.clone(),
                source,
            })?,
        )
    } else {
        None
    };
    let mut blobs = None;
    for entry in tracked_entries {
        validate_relative_path(&entry.path)?;
        let destination = head_path.join(&entry.path);
        match entry.mode.as_str() {
            "100644" | "100755" => {
                create_parent(&destination)?;
                if materialize_regular_file(
                    repository,
                    &mut blobs,
                    heads_directory,
                    entry,
                    &destination,
                    reusable_source_root.as_deref(),
                    force_full_copy,
                )? == StorageBackend::FullCopy
                {
                    backend = StorageBackend::FullCopy;
                }
                set_executable(&destination, entry.mode == "100755")?;
            }
            "120000" => materialize_symlink(repository, &mut blobs, entry, &destination)?,
            "160000" => create_directory(&destination)?,
            _ => {
                return Err(HeadError::UnsupportedTrackedEntry {
                    mode: entry.mode.clone(),
                    path: entry.path.clone(),
                });
            }
        }
    }
    if let Some(blobs) = blobs {
        blobs.finish()?;
    }
    Ok(backend)
}

fn materialize_regular_file(
    repository: &Repository,
    blobs: &mut Option<GitBlobBatch>,
    heads_directory: &Path,
    entry: &TrackedEntry,
    destination: &Path,
    reusable_source_root: Option<&Path>,
    force_full_copy: bool,
) -> Result<StorageBackend, HeadError> {
    if !force_full_copy
        && let Some(canonical_source_root) = reusable_source_root
        && try_reuse_tracked_source(repository, canonical_source_root, entry, destination)?
    {
        return Ok(StorageBackend::CopyOnWrite);
    }
    let temporary_path = heads_directory.join(format!(
        ".hydra-blob-{}-{}",
        entry.object,
        Uuid::new_v4().simple()
    ));
    let blobs = blob_batch(repository, blobs)?;
    write_blob_to_file(blobs, &entry.object, &temporary_path)?;

    let backend = if force_full_copy {
        copy_exclusive(&temporary_path, destination)
            .map(|()| StorageBackend::FullCopy)
            .map_err(|source| HeadError::FileSystem {
                action: "copy tracked file",
                path: destination.to_path_buf(),
                source,
            })
    } else {
        match reflink_copy::reflink(&temporary_path, destination) {
            Ok(()) => Ok(StorageBackend::CopyOnWrite),
            Err(_) => copy_exclusive(&temporary_path, destination)
                .map(|()| StorageBackend::FullCopy)
                .map_err(|source| HeadError::FileSystem {
                    action: "copy tracked file",
                    path: destination.to_path_buf(),
                    source,
                }),
        }
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

fn try_reuse_tracked_source(
    repository: &Repository,
    canonical_source_root: &Path,
    entry: &TrackedEntry,
    destination: &Path,
) -> Result<bool, HeadError> {
    let source = repository.root.join(&entry.path);
    let metadata = match fs::symlink_metadata(&source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(source_error) => {
            return Err(HeadError::FileSystem {
                action: "inspect reusable tracked source",
                path: source,
                source: source_error,
            });
        }
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Ok(false);
    }
    let canonical_source = match fs::canonicalize(&source) {
        Ok(canonical_source) => canonical_source,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(source_error) => {
            return Err(HeadError::FileSystem {
                action: "resolve reusable tracked source",
                path: source,
                source: source_error,
            });
        }
    };
    if !canonical_source.starts_with(canonical_source_root) {
        return Err(HeadError::UnsafeProjectFile(source));
    }
    if reflink_copy::reflink(&source, destination).is_ok() {
        Ok(true)
    } else {
        remove_failed_reflink_destination(destination)?;
        Ok(false)
    }
}

fn blob_batch<'a>(
    repository: &Repository,
    blobs: &'a mut Option<GitBlobBatch>,
) -> Result<&'a mut GitBlobBatch, HeadError> {
    if blobs.is_none() {
        *blobs = Some(GitBlobBatch::start(repository)?);
    }
    blobs
        .as_mut()
        .ok_or(HeadError::InvalidGitOutput("tracked blob batch process"))
}

fn write_blob_to_file(
    blobs: &mut GitBlobBatch,
    object: &str,
    destination: &Path,
) -> Result<(), HeadError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|source| HeadError::FileSystem {
            action: "create temporary Git blob",
            path: destination.to_path_buf(),
            source,
        })?;
    if let Err(error) = blobs.write_blob(object, &mut file, destination) {
        return Err(remove_temporary_blob_after_error(destination, error));
    }
    if let Err(source) = file.sync_all() {
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

fn remove_failed_reflink_destination(destination: &Path) -> Result<(), HeadError> {
    match fs::remove_file(destination) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(HeadError::FileSystem {
            action: "remove failed tracked reflink",
            path: destination.to_path_buf(),
            source,
        }),
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
    blobs: &mut Option<GitBlobBatch>,
    entry: &TrackedEntry,
    destination: &Path,
) -> Result<(), HeadError> {
    create_parent(destination)?;
    let blobs = blob_batch(repository, blobs)?;
    let target = blobs.read_blob(&entry.object)?;
    symlink(OsString::from_vec(target), destination).map_err(|source| HeadError::FileSystem {
        action: "create tracked symlink",
        path: destination.to_path_buf(),
        source,
    })
}

#[cfg(not(unix))]
fn materialize_symlink(
    _repository: &Repository,
    _blobs: &mut Option<GitBlobBatch>,
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

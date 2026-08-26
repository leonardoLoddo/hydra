use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use super::{
    OverlayFile, OverlayKind, OverlayPlan, hash, validate_relative_path,
    validate_symlink_resolution,
};
use crate::{
    StorageBackend,
    head::{HeadError, git::Repository},
};

pub(in crate::head) fn materialize_overlays(
    repository: &Repository,
    plan: &OverlayPlan,
    head_path: &Path,
    confirmed_full_copy: bool,
    force_full_copy: bool,
) -> Result<StorageBackend, HeadError> {
    let mut backend = StorageBackend::CopyOnWrite;
    let canonical_repository_root =
        crate::path::canonicalize(&repository.root).map_err(|source| HeadError::FileSystem {
            action: "resolve overlay source root",
            path: repository.root.clone(),
            source,
        })?;
    create_overlay_parents(head_path, &plan.files)?;
    let regular_files = plan
        .files
        .iter()
        .filter(|file| matches!(file.kind, OverlayKind::Regular { .. }))
        .collect::<Vec<_>>();
    for file in &regular_files {
        let OverlayKind::Regular {
            identity: _,
            requires_full_copy: _,
        } = &file.kind
        else {
            unreachable!("filtered overlay kind should be regular");
        };
        validate_regular_overlay_source(&canonical_repository_root, &file.source)?;
        let destination = head_path.join(&file.relative);

        let file_backend = if !force_full_copy
            && reflink_copy::reflink(&file.source, &destination).is_ok()
        {
            StorageBackend::CopyOnWrite
        } else {
            remove_failed_reflink_destination(&destination)?;
            if !confirmed_full_copy {
                return Err(HeadError::OverlayFullCopyConfirmationRequired {
                    files: 1,
                    bytes: file.size,
                });
            }
            copy_exclusive(&file.source, &destination).map_err(|source| HeadError::FileSystem {
                action: "copy overlay file",
                path: destination.clone(),
                source,
            })?;
            StorageBackend::FullCopy
        };
        if file_backend == StorageBackend::FullCopy {
            backend = StorageBackend::FullCopy;
        }

        let permissions = fs::metadata(&file.source)
            .map_err(|source| HeadError::FileSystem {
                action: "read overlay permissions",
                path: file.source.clone(),
                source,
            })?
            .permissions();
        fs::set_permissions(&destination, permissions).map_err(|source| HeadError::FileSystem {
            action: "set overlay permissions",
            path: destination.clone(),
            source,
        })?;
    }
    verify_regular_overlay_identities(&repository.root, head_path, regular_files.as_slice())?;

    for file in &plan.files {
        let OverlayKind::Symlink { target } = &file.kind else {
            continue;
        };
        validate_symlink_overlay_source(&canonical_repository_root, &file.source, target)?;
        let destination = head_path.join(&file.relative);
        create_overlay_symlink(target, &destination)?;
    }

    for file in &plan.files {
        let OverlayKind::Symlink { target } = &file.kind else {
            continue;
        };
        validate_materialized_symlink(head_path, &head_path.join(&file.relative), target)?;
    }

    Ok(backend)
}

pub(super) fn verify_regular_overlay_identities(
    repository_root: &Path,
    head_path: &Path,
    files: &[&OverlayFile],
) -> Result<(), HeadError> {
    let mut paths = Vec::with_capacity(files.len());
    for file in files {
        paths.push(head_path.join(&file.relative));
    }
    let hashes = hash::hash_paths(repository_root, &paths)?;
    for (file, materialized_identity) in files.iter().zip(hashes) {
        let OverlayKind::Regular {
            identity: Some(identity),
            ..
        } = &file.kind
        else {
            return Err(HeadError::InvalidGitOutput("overlay identity"));
        };
        if materialized_identity != *identity {
            return Err(HeadError::OverlayChanged(file.source.clone()));
        }
    }
    Ok(())
}

fn remove_failed_reflink_destination(destination: &Path) -> Result<(), HeadError> {
    match fs::remove_file(destination) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(HeadError::FileSystem {
            action: "remove failed overlay reflink",
            path: destination.to_path_buf(),
            source,
        }),
    }
}

pub(super) fn overlay_parent_paths(files: &[OverlayFile]) -> Result<Vec<PathBuf>, HeadError> {
    let mut parents = BTreeSet::new();
    for file in files {
        let parent = file
            .relative
            .parent()
            .ok_or_else(|| HeadError::UnsafeOverlayPath(file.relative.clone()))?;
        if !parent.as_os_str().is_empty() {
            validate_relative_path(parent)?;
            parents.insert(parent.to_path_buf());
        }
    }
    Ok(parents.into_iter().collect())
}

fn create_overlay_parents(head_path: &Path, files: &[OverlayFile]) -> Result<(), HeadError> {
    for relative in overlay_parent_paths(files)? {
        let parent = head_path.join(&relative);
        fs::create_dir_all(&parent).map_err(|source| HeadError::FileSystem {
            action: "create overlay parent directory",
            path: parent,
            source,
        })?;
    }
    Ok(())
}

fn validate_symlink_overlay_source(
    canonical_repository_root: &Path,
    source: &Path,
    expected_target: &Path,
) -> Result<(), HeadError> {
    let metadata = fs::symlink_metadata(source).map_err(|source_error| HeadError::FileSystem {
        action: "revalidate overlay symlink",
        path: source.to_path_buf(),
        source: source_error,
    })?;
    if !metadata.file_type().is_symlink() {
        return Err(HeadError::UnsafeOverlayPath(source.to_path_buf()));
    }
    let target = fs::read_link(source).map_err(|source_error| HeadError::FileSystem {
        action: "reread overlay symlink",
        path: source.to_path_buf(),
        source: source_error,
    })?;
    if target != expected_target {
        return Err(HeadError::OverlayChanged(source.to_path_buf()));
    }
    validate_symlink_resolution(canonical_repository_root, source, &target)
}

#[cfg(unix)]
fn create_overlay_symlink(target: &Path, destination: &Path) -> Result<(), HeadError> {
    std::os::unix::fs::symlink(target, destination).map_err(|source| HeadError::FileSystem {
        action: "create overlay symlink",
        path: destination.to_path_buf(),
        source,
    })
}

#[cfg(not(unix))]
fn create_overlay_symlink(_target: &Path, destination: &Path) -> Result<(), HeadError> {
    Err(HeadError::UnsafeOverlayPath(destination.to_path_buf()))
}

fn validate_materialized_symlink(
    head_root: &Path,
    destination: &Path,
    expected_target: &Path,
) -> Result<(), HeadError> {
    let target = fs::read_link(destination).map_err(|source_error| HeadError::FileSystem {
        action: "read materialized overlay symlink",
        path: destination.to_path_buf(),
        source: source_error,
    })?;
    if target != expected_target {
        return Err(HeadError::OverlayChanged(destination.to_path_buf()));
    }
    let canonical_head =
        crate::path::canonicalize(head_root).map_err(|source_error| HeadError::FileSystem {
            action: "resolve materialized Head root",
            path: head_root.to_path_buf(),
            source: source_error,
        })?;
    let canonical_target = crate::path::canonicalize(destination)
        .map_err(|_| HeadError::UnsafeOverlayPath(destination.to_path_buf()))?;
    if canonical_target.starts_with(canonical_head) {
        Ok(())
    } else {
        Err(HeadError::UnsafeOverlayPath(destination.to_path_buf()))
    }
}

fn validate_regular_overlay_source(
    canonical_repository_root: &Path,
    source: &Path,
) -> Result<(), HeadError> {
    let metadata = fs::symlink_metadata(source).map_err(|source_error| HeadError::FileSystem {
        action: "revalidate overlay source",
        path: source.to_path_buf(),
        source: source_error,
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(HeadError::UnsafeOverlayPath(source.to_path_buf()));
    }
    let canonical_source =
        crate::path::canonicalize(source).map_err(|source_error| HeadError::FileSystem {
            action: "resolve overlay source",
            path: source.to_path_buf(),
            source: source_error,
        })?;
    if canonical_source.starts_with(canonical_repository_root) {
        Ok(())
    } else {
        Err(HeadError::UnsafeOverlayPath(source.to_path_buf()))
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

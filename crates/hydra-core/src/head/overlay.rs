use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Component, Path, PathBuf},
    process::Command,
};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

use super::{
    HeadError,
    git::{Repository, TrackedEntry},
};
use crate::StorageBackend;

#[derive(Debug)]
struct OverlayFile {
    source: PathBuf,
    relative: PathBuf,
    size: u64,
    identity: String,
}

#[derive(Debug)]
pub(super) struct OverlayPlan {
    files: Vec<OverlayFile>,
    total_bytes: u64,
}

impl OverlayPlan {
    pub(super) fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub(super) fn file_count(&self) -> usize {
        self.files.len()
    }

    pub(super) fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
}

pub(super) fn plan_overlays(
    source_root: &Path,
    rules: &[String],
    tracked_entries: &[TrackedEntry],
) -> Result<OverlayPlan, HeadError> {
    let matcher = build_matcher(source_root, rules)?;
    let tracked = tracked_entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<HashSet<_>>();
    let mut files = Vec::new();
    visit_directory(source_root, source_root, &matcher, &tracked, &mut files)?;
    files.sort_by(|left, right| left.relative.cmp(&right.relative));
    let total_bytes = files.iter().try_fold(0_u64, |total, file| {
        total
            .checked_add(file.size)
            .ok_or_else(|| HeadError::OverlayRules("overlay size overflow".to_owned()))
    })?;
    Ok(OverlayPlan { files, total_bytes })
}

pub(super) fn materialize_overlays(
    repository: &Repository,
    plan: &OverlayPlan,
    head_path: &Path,
) -> Result<StorageBackend, HeadError> {
    let mut backend = StorageBackend::CopyOnWrite;
    for file in &plan.files {
        validate_overlay_source(&repository.root, &file.source)?;
        let destination = head_path.join(&file.relative);
        let parent = destination
            .parent()
            .ok_or_else(|| HeadError::UnsafeOverlayPath(file.relative.clone()))?;
        fs::create_dir_all(parent).map_err(|source| HeadError::FileSystem {
            action: "create overlay parent directory",
            path: parent.to_path_buf(),
            source,
        })?;

        let file_backend = if reflink_copy::reflink(&file.source, &destination).is_ok() {
            StorageBackend::CopyOnWrite
        } else {
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

        if hash_file(&repository.root, &file.source)? != file.identity
            || hash_file(&repository.root, &destination)? != file.identity
        {
            return Err(HeadError::OverlayChanged(file.source.clone()));
        }
    }
    Ok(backend)
}

fn build_matcher(source_root: &Path, rules: &[String]) -> Result<Gitignore, HeadError> {
    let mut builder = GitignoreBuilder::new(source_root);
    for rule in rules {
        if let Some(included) = rule.strip_prefix("... ") {
            let relative = Path::new(included);
            validate_relative_path(relative)?;
            let path = source_root.join(relative);
            match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_file() => {
                    if let Some(error) = builder.add(&path) {
                        return Err(HeadError::OverlayRules(error.to_string()));
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Ok(_) => return Err(HeadError::UnsafeOverlayPath(path)),
                Err(source) => {
                    return Err(HeadError::FileSystem {
                        action: "inspect included overlay rules",
                        path,
                        source,
                    });
                }
            }
        } else {
            builder
                .add_line(None, rule)
                .map_err(|error| HeadError::OverlayRules(error.to_string()))?;
        }
    }
    builder
        .build()
        .map_err(|error| HeadError::OverlayRules(error.to_string()))
}

fn visit_directory(
    source_root: &Path,
    directory: &Path,
    matcher: &Gitignore,
    tracked: &HashSet<PathBuf>,
    files: &mut Vec<OverlayFile>,
) -> Result<(), HeadError> {
    let mut entries = fs::read_dir(directory)
        .map_err(|source| HeadError::FileSystem {
            action: "scan overlay source directory",
            path: directory.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| HeadError::FileSystem {
            action: "read overlay source entry",
            path: directory.to_path_buf(),
            source,
        })?;
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let source = entry.path();
        let relative = source
            .strip_prefix(source_root)
            .map_err(|_| HeadError::UnsafeOverlayPath(source.clone()))?
            .to_path_buf();
        validate_relative_path(&relative)?;
        if relative
            .components()
            .next()
            .is_some_and(|component| component.as_os_str() == ".git")
        {
            continue;
        }

        let metadata =
            fs::symlink_metadata(&source).map_err(|source_error| HeadError::FileSystem {
                action: "inspect overlay source",
                path: source.clone(),
                source: source_error,
            })?;
        if metadata.is_dir() {
            visit_directory(source_root, &source, matcher, tracked, files)?;
        } else if matcher
            .matched_path_or_any_parents(&relative, false)
            .is_ignore()
        {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(HeadError::UnsafeOverlayPath(source));
            }
            if tracked.contains(&relative) {
                return Err(HeadError::OverlayOverwritesTracked(relative));
            }
            files.push(OverlayFile {
                identity: hash_file(source_root, &source)?,
                source,
                relative,
                size: metadata.len(),
            });
        }
    }
    Ok(())
}

fn validate_relative_path(path: &Path) -> Result<(), HeadError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        Err(HeadError::UnsafeOverlayPath(path.to_path_buf()))
    } else {
        Ok(())
    }
}

fn hash_file(repository_root: &Path, path: &Path) -> Result<String, HeadError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(["hash-object", "--no-filters", "--"])
        .arg(path)
        .output()
        .map_err(HeadError::GitUnavailable)?;
    if !output.status.success() {
        return Err(HeadError::GitCommandFailed {
            operation: "hashing an overlay file",
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let value = String::from_utf8(output.stdout)
        .map_err(|_| HeadError::InvalidGitOutput("overlay identity"))?;
    let value = value.trim_end_matches(['\r', '\n']);
    if value.is_empty() {
        Err(HeadError::InvalidGitOutput("overlay identity"))
    } else {
        Ok(value.to_owned())
    }
}

fn validate_overlay_source(repository_root: &Path, source: &Path) -> Result<(), HeadError> {
    let metadata = fs::symlink_metadata(source).map_err(|source_error| HeadError::FileSystem {
        action: "revalidate overlay source",
        path: source.to_path_buf(),
        source: source_error,
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(HeadError::UnsafeOverlayPath(source.to_path_buf()));
    }
    let canonical_root =
        fs::canonicalize(repository_root).map_err(|source_error| HeadError::FileSystem {
            action: "resolve overlay source root",
            path: repository_root.to_path_buf(),
            source: source_error,
        })?;
    let canonical_source =
        fs::canonicalize(source).map_err(|source_error| HeadError::FileSystem {
            action: "resolve overlay source",
            path: source.to_path_buf(),
            source: source_error,
        })?;
    if canonical_source.starts_with(canonical_root) {
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

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use super::{super::HeadError, configuration::ProjectConfiguration};
use crate::head::git::{self, Repository};

const LOCATOR_FILE_NAME: &str = "project.json";
const DIRECTORY_MARKER_FILE_NAME: &str = "directory.json";
const STATE_FILE_NAME: &str = "heads.json";
const SUPPORTED_LOCAL_METADATA_VERSION: u32 = 1;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectLocator {
    version: u32,
    project_id: String,
    installation_id: String,
    project_root: PathBuf,
    heads_directory: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DirectoryMarker {
    version: u32,
    project_id: String,
    installation_id: String,
}

pub(super) fn discover_project_repository(source_path: &Path) -> Result<Repository, HeadError> {
    let source_repository = Repository::discover(source_path)?;
    let locator_path = source_repository
        .git_common_directory
        .join("hydra")
        .join(LOCATOR_FILE_NAME);
    if let Err(source) = fs::symlink_metadata(&locator_path) {
        if source.kind() == std::io::ErrorKind::NotFound {
            return Err(HeadError::ProjectNotInitialized(
                source_repository.root.join(".hydra.json"),
            ));
        }
        return Err(HeadError::FileSystem {
            action: "inspect Hydra project file",
            path: locator_path,
            source,
        });
    }
    validate_regular_file(&locator_path)?;
    let locator: ProjectLocator = read_local_metadata(
        &locator_path,
        "project locator",
        "read local project locator",
    )?;
    validate_local_metadata_version("project locator", locator.version)?;
    if !locator.project_root.is_absolute() {
        return Err(HeadError::UnsafeHeadsDirectory(locator.project_root));
    }
    validate_real_directory(&locator.project_root)?;

    let project_repository = Repository::discover(&locator.project_root)?;
    let source_common = canonicalize_git_directory(&source_repository.git_common_directory)?;
    let project_common = canonicalize_git_directory(&project_repository.git_common_directory)?;
    if source_common != project_common {
        return Err(HeadError::LocalIdentityMismatch(locator_path));
    }
    Ok(project_repository)
}

fn canonicalize_git_directory(path: &Path) -> Result<PathBuf, HeadError> {
    fs::canonicalize(path).map_err(|source| HeadError::FileSystem {
        action: "resolve Git common directory",
        path: path.to_path_buf(),
        source,
    })
}

pub(super) fn inventory_path(
    configuration: &ProjectConfiguration,
    repository: &Repository,
) -> Result<PathBuf, HeadError> {
    let locator_path = repository
        .git_common_directory
        .join("hydra")
        .join(LOCATOR_FILE_NAME);
    validate_regular_file(&locator_path)?;
    let locator: ProjectLocator = read_local_metadata(
        &locator_path,
        "project locator",
        "read local project locator",
    )?;
    validate_local_metadata_version("project locator", locator.version)?;
    if locator.project_id != configuration.project_id() {
        return Err(HeadError::LocalIdentityMismatch(locator_path));
    }

    let heads_directory = validate_heads_directory(configuration, &locator, repository)?;
    let metadata_directory = heads_directory.join(".hydra");
    validate_real_directory(&metadata_directory)?;
    let marker_path = metadata_directory.join(DIRECTORY_MARKER_FILE_NAME);
    validate_regular_file(&marker_path)?;
    let marker: DirectoryMarker = read_local_metadata(
        &marker_path,
        "directory ownership marker",
        "read directory ownership marker",
    )?;
    validate_local_metadata_version("directory ownership marker", marker.version)?;
    if marker.project_id != locator.project_id || marker.installation_id != locator.installation_id
    {
        return Err(HeadError::LocalIdentityMismatch(marker_path));
    }

    let inventory_path = metadata_directory.join(STATE_FILE_NAME);
    validate_regular_file(&inventory_path)?;
    Ok(inventory_path)
}

fn validate_heads_directory(
    configuration: &ProjectConfiguration,
    locator: &ProjectLocator,
    repository: &Repository,
) -> Result<PathBuf, HeadError> {
    if !locator.project_root.is_absolute() || !locator.heads_directory.is_absolute() {
        return Err(HeadError::UnsafeHeadsDirectory(
            locator.heads_directory.clone(),
        ));
    }
    validate_real_directory(&locator.project_root)?;
    validate_real_directory(&locator.heads_directory)?;

    let project_root =
        fs::canonicalize(&locator.project_root).map_err(|source| HeadError::FileSystem {
            action: "resolve local project root",
            path: locator.project_root.clone(),
            source,
        })?;
    let heads =
        fs::canonicalize(&locator.heads_directory).map_err(|source| HeadError::FileSystem {
            action: "resolve Heads directory",
            path: locator.heads_directory.clone(),
            source,
        })?;
    let repository_root =
        fs::canonicalize(&repository.root).map_err(|source| HeadError::FileSystem {
            action: "resolve repository root",
            path: repository.root.clone(),
            source,
        })?;
    let expected = configuration.resolve_heads_directory(&project_root, &heads)?;
    let expected = match fs::canonicalize(&expected) {
        Ok(expected) => expected,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Err(HeadError::DirectoryPolicyMismatch(heads));
        }
        Err(source) => {
            return Err(HeadError::FileSystem {
                action: "resolve configured Heads directory",
                path: expected,
                source,
            });
        }
    };
    if expected != heads {
        return Err(HeadError::DirectoryPolicyMismatch(heads));
    }

    if heads.starts_with(&project_root) || heads.starts_with(&repository_root) {
        return Err(HeadError::UnsafeHeadsDirectory(heads));
    }
    for worktree in git::worktree_paths(repository)? {
        match fs::canonicalize(&worktree) {
            Ok(worktree) if heads.starts_with(&worktree) => {
                return Err(HeadError::UnsafeHeadsDirectory(heads));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(HeadError::FileSystem {
                    action: "resolve registered Git worktree",
                    path: worktree,
                    source,
                });
            }
        }
    }
    Ok(heads)
}

fn validate_real_directory(path: &Path) -> Result<(), HeadError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| HeadError::FileSystem {
        action: "inspect Hydra directory",
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err(HeadError::UnsafeHeadsDirectory(path.to_path_buf()))
    }
}

fn read_local_metadata<T>(
    path: &Path,
    kind: &'static str,
    action: &'static str,
) -> Result<T, HeadError>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes = fs::read(path).map_err(|source| HeadError::FileSystem {
        action,
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|source| HeadError::InvalidLocalMetadata {
        kind,
        path: path.to_path_buf(),
        source,
    })
}

fn validate_local_metadata_version(kind: &'static str, version: u32) -> Result<(), HeadError> {
    if version == SUPPORTED_LOCAL_METADATA_VERSION {
        Ok(())
    } else {
        Err(HeadError::UnsupportedLocalMetadataVersion { kind, version })
    }
}

fn validate_regular_file(path: &Path) -> Result<(), HeadError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => Err(HeadError::UnsafeProjectFile(path.to_path_buf())),
        Err(source) => Err(HeadError::FileSystem {
            action: "inspect Hydra project file",
            path: path.to_path_buf(),
            source,
        }),
    }
}

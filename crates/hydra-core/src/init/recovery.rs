use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use super::{
    DIRECTORY_MARKER_FILE_NAME, HEADS_METADATA_DIRECTORY_NAME, InitError, LOCATOR_FILE_NAME,
    STATE_FILE_NAME, SUPPORTED_LOCAL_METADATA_VERSION,
};

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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeadInventory {
    version: u32,
    heads: BTreeMap<String, serde_json::Value>,
}

pub(super) struct ExistingInstallation {
    project_id: String,
    evidence: Vec<(PathBuf, Vec<u8>)>,
    heads_directory: PathBuf,
    heads_metadata_directory: PathBuf,
    state_directory: PathBuf,
}

impl ExistingInstallation {
    pub(super) fn project_id(&self) -> &str {
        &self.project_id
    }

    pub(super) fn verify_unchanged(&self) -> Result<(), InitError> {
        validate_reusable_directory_contents(
            &self.heads_directory,
            &[HEADS_METADATA_DIRECTORY_NAME],
        )?;
        validate_reusable_directory_contents(
            &self.heads_metadata_directory,
            &[DIRECTORY_MARKER_FILE_NAME, STATE_FILE_NAME],
        )?;
        validate_reusable_directory_contents(&self.state_directory, &[LOCATOR_FILE_NAME])?;
        for (path, expected) in &self.evidence {
            let current = read_regular_file(path, "existing Hydra recovery metadata")?;
            if current != *expected {
                return Err(InitError::ExistingInstallationChanged(path.clone()));
            }
        }
        Ok(())
    }
}

pub(super) struct ExistingInstallationPaths<'a> {
    pub(super) repository_root: &'a Path,
    pub(super) heads_directory: &'a Path,
    pub(super) state_directory: &'a Path,
    pub(super) locator_path: &'a Path,
    pub(super) heads_metadata_directory: &'a Path,
    pub(super) marker_path: &'a Path,
    pub(super) inventory_path: &'a Path,
}

pub(super) fn load_existing_installation(
    paths: &ExistingInstallationPaths<'_>,
) -> Result<Option<ExistingInstallation>, InitError> {
    if !is_real_directory(paths.heads_directory, "inspect Heads directory")?
        || !is_real_directory(paths.state_directory, "inspect local state directory")?
    {
        return Ok(None);
    }
    if !is_real_directory(
        paths.heads_metadata_directory,
        "inspect Heads metadata directory",
    )? {
        return Err(InitError::ExistingInstallationIncomplete(
            paths.heads_metadata_directory.to_path_buf(),
        ));
    }

    let locator_bytes = read_regular_file(paths.locator_path, "project locator")?;
    let marker_bytes = read_regular_file(paths.marker_path, "directory ownership marker")?;
    let inventory_bytes = read_regular_file(paths.inventory_path, "Head inventory")?;
    let locator: ProjectLocator =
        deserialize_local_metadata(&locator_bytes, "project locator", paths.locator_path)?;
    let marker: DirectoryMarker = deserialize_local_metadata(
        &marker_bytes,
        "directory ownership marker",
        paths.marker_path,
    )?;
    let inventory: HeadInventory =
        deserialize_local_metadata(&inventory_bytes, "Head inventory", paths.inventory_path)?;
    validate_local_metadata_version("project locator", locator.version)?;
    validate_local_metadata_version("directory ownership marker", marker.version)?;
    validate_local_metadata_version("Head inventory", inventory.version)?;

    if !locator.project_root.is_absolute() || !locator.heads_directory.is_absolute() {
        return Err(InitError::ExistingOwnershipMismatch(
            paths.locator_path.to_path_buf(),
        ));
    }
    let located_project = canonicalize_existing_path(&locator.project_root)?;
    let located_heads = canonicalize_existing_path(&locator.heads_directory)?;
    let expected_heads = canonicalize_existing_path(paths.heads_directory)?;
    if located_project != paths.repository_root
        || located_heads != expected_heads
        || locator.project_id != marker.project_id
        || locator.installation_id != marker.installation_id
    {
        return Err(InitError::ExistingOwnershipMismatch(
            paths.marker_path.to_path_buf(),
        ));
    }
    if !inventory.heads.is_empty() {
        return Err(InitError::ExistingHeadsRequireConfiguration(
            paths.inventory_path.to_path_buf(),
        ));
    }

    validate_reusable_directory_contents(paths.heads_directory, &[HEADS_METADATA_DIRECTORY_NAME])?;
    validate_reusable_directory_contents(
        paths.heads_metadata_directory,
        &[DIRECTORY_MARKER_FILE_NAME, STATE_FILE_NAME],
    )?;
    validate_reusable_directory_contents(paths.state_directory, &[LOCATOR_FILE_NAME])?;

    Ok(Some(ExistingInstallation {
        project_id: locator.project_id,
        evidence: vec![
            (paths.locator_path.to_path_buf(), locator_bytes),
            (paths.marker_path.to_path_buf(), marker_bytes),
            (paths.inventory_path.to_path_buf(), inventory_bytes),
        ],
        heads_directory: paths.heads_directory.to_path_buf(),
        heads_metadata_directory: paths.heads_metadata_directory.to_path_buf(),
        state_directory: paths.state_directory.to_path_buf(),
    }))
}

fn validate_reusable_directory_contents(
    directory: &Path,
    allowed_names: &[&str],
) -> Result<(), InitError> {
    let entries = fs::read_dir(directory).map_err(|source| InitError::FileSystem {
        action: "read existing Hydra directory",
        path: directory.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| InitError::FileSystem {
            action: "read existing Hydra directory entry",
            path: directory.to_path_buf(),
            source,
        })?;
        if !allowed_names
            .iter()
            .any(|allowed| entry.file_name() == OsStr::new(allowed))
        {
            return Err(InitError::ExistingHeadsRequireConfiguration(entry.path()));
        }
    }
    Ok(())
}

fn is_real_directory(path: &Path, action: &'static str) -> Result<bool, InitError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.is_dir() && !metadata.file_type().is_symlink()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(InitError::FileSystem {
            action,
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn read_regular_file(path: &Path, kind: &'static str) -> Result<Vec<u8>, InitError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {}
        Ok(_) => {
            return Err(InitError::ExistingInstallationIncomplete(
                path.to_path_buf(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(InitError::ExistingInstallationIncomplete(
                path.to_path_buf(),
            ));
        }
        Err(source) => {
            return Err(InitError::FileSystem {
                action: "inspect existing Hydra metadata",
                path: path.to_path_buf(),
                source,
            });
        }
    }
    fs::read(path).map_err(|source| InitError::FileSystem {
        action: kind,
        path: path.to_path_buf(),
        source,
    })
}

fn deserialize_local_metadata<T>(
    bytes: &[u8],
    kind: &'static str,
    path: &Path,
) -> Result<T, InitError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_slice(bytes).map_err(|source| InitError::InvalidLocalMetadata {
        kind,
        path: path.to_path_buf(),
        source,
    })
}

fn validate_local_metadata_version(kind: &'static str, version: u32) -> Result<(), InitError> {
    if version == SUPPORTED_LOCAL_METADATA_VERSION {
        Ok(())
    } else {
        Err(InitError::UnsupportedLocalMetadataVersion { kind, version })
    }
}

fn canonicalize_existing_path(path: &Path) -> Result<PathBuf, InitError> {
    fs::canonicalize(path).map_err(|source| InitError::FileSystem {
        action: "resolve existing Hydra path",
        path: path.to_path_buf(),
        source,
    })
}

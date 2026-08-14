use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{
    HeadError,
    git::{self, Repository},
    persistence::create_file_atomically,
    state::HeadMetadata,
    validate_head_name,
};

const RECOVERY_FILE_NAME: &str = "hydra-head.json";
const RECOVERY_VERSION: u32 = 1;
const CENTRAL_RECOVERY_PREFIX: &str = "recovery-";
const PENDING_PREFIX: &str = "pending-";
const PENDING_SUFFIX: &str = ".json";

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecoveryRecord {
    version: u32,
    name: String,
    metadata: HeadMetadata,
}

pub(super) struct RecoveredHead {
    pub(super) name: String,
    pub(super) metadata: HeadMetadata,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PendingCreationIntent {
    worktree_path: PathBuf,
    head_ref: String,
    base_ref: String,
    base_commit: String,
    target_ref: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PendingCreationRecord {
    version: u32,
    name: String,
    intent: PendingCreationIntent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    metadata: Option<HeadMetadata>,
}

pub(super) struct PendingCreation {
    path: PathBuf,
    name: String,
    intent: PendingCreationIntent,
}

impl PendingCreationIntent {
    pub(super) fn worktree_path(&self) -> &Path {
        &self.worktree_path
    }

    pub(super) fn head_ref(&self) -> &str {
        &self.head_ref
    }

    pub(super) fn base_commit(&self) -> &str {
        &self.base_commit
    }
}

impl PendingCreation {
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn intent(&self) -> &PendingCreationIntent {
        &self.intent
    }
}

pub(super) fn create_pending_creation(
    heads_directory: &Path,
    name: &str,
    worktree_path: &Path,
    head_ref: &str,
    base_ref: &str,
    base_commit: &str,
    target_ref: &str,
) -> Result<PendingCreation, HeadError> {
    let intent = PendingCreationIntent {
        worktree_path: worktree_path.to_path_buf(),
        head_ref: head_ref.to_owned(),
        base_ref: base_ref.to_owned(),
        base_commit: base_commit.to_owned(),
        target_ref: target_ref.to_owned(),
    };
    let record = PendingCreationRecord {
        version: RECOVERY_VERSION,
        name: name.to_owned(),
        intent: intent.clone(),
        metadata: None,
    };
    let path = pending_creation_path(heads_directory, name);
    let mut contents = serde_json::to_vec_pretty(&record).map_err(HeadError::SerializeState)?;
    contents.push(b'\n');
    create_file_atomically(&path, &contents)?;
    Ok(PendingCreation {
        path,
        name: name.to_owned(),
        intent,
    })
}

pub(super) fn read_pending_creations(
    heads_directory: &Path,
) -> Result<Vec<PendingCreation>, HeadError> {
    let metadata_directory = heads_directory.join(".hydra");
    let entries = fs::read_dir(&metadata_directory).map_err(|source| HeadError::FileSystem {
        action: "list pending Head creations",
        path: metadata_directory,
        source,
    })?;
    let mut pending = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| HeadError::FileSystem {
            action: "read pending Head creation entry",
            path: heads_directory.join(".hydra"),
            source,
        })?;
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        let Some(name) = file_name
            .strip_prefix(PENDING_PREFIX)
            .and_then(|name| name.strip_suffix(PENDING_SUFFIX))
        else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        pending.push(read_pending_creation(entry.path(), name)?);
    }
    pending.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(pending)
}

pub(super) fn remove_pending_creation(path: &Path) -> Result<(), HeadError> {
    fs::remove_file(path).map_err(|source| HeadError::FileSystem {
        action: "remove pending Head creation",
        path: path.to_path_buf(),
        source,
    })
}

fn read_pending_creation(path: PathBuf, expected_name: &str) -> Result<PendingCreation, HeadError> {
    let metadata = fs::symlink_metadata(&path).map_err(|source| HeadError::FileSystem {
        action: "inspect pending Head creation",
        path: path.clone(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(HeadError::UnsafeProjectFile(path));
    }
    let contents = fs::read(&path).map_err(|source| HeadError::FileSystem {
        action: "read pending Head creation",
        path: path.clone(),
        source,
    })?;
    let record: PendingCreationRecord =
        serde_json::from_slice(&contents).map_err(|source| HeadError::InvalidLocalMetadata {
            kind: "pending Head creation",
            path: path.clone(),
            source,
        })?;
    if record.version != RECOVERY_VERSION {
        return Err(HeadError::UnsupportedLocalMetadataVersion {
            kind: "pending Head creation",
            version: record.version,
        });
    }
    if record.name != expected_name {
        return Err(HeadError::InvalidGitOutput("pending Head creation name"));
    }
    Ok(PendingCreation {
        path,
        name: record.name,
        intent: record.intent,
    })
}

fn pending_creation_path(heads_directory: &Path, name: &str) -> PathBuf {
    heads_directory
        .join(".hydra")
        .join(format!("{PENDING_PREFIX}{name}{PENDING_SUFFIX}"))
}

pub(super) fn create_manifest(
    repository: &Repository,
    worktree: &Path,
    name: &str,
    metadata: &HeadMetadata,
) -> Result<(), HeadError> {
    let path = git::worktree_private_file(repository, worktree, RECOVERY_FILE_NAME)?;
    let contents = serialize_recovery_record(name, metadata)?;
    create_file_atomically(&path, &contents)
}

pub(super) fn create_central_recovery(
    heads_directory: &Path,
    name: &str,
    metadata: &HeadMetadata,
) -> Result<PathBuf, HeadError> {
    let path = central_recovery_path(heads_directory, name)?;
    let contents = serialize_recovery_record(name, metadata)?;
    create_file_atomically(&path, &contents)?;
    Ok(path)
}

pub(super) fn read_central_recovery(
    heads_directory: &Path,
    name: &str,
) -> Result<Option<RecoveredHead>, HeadError> {
    read_recovery_record(
        &central_recovery_path(heads_directory, name)?,
        "central Head recovery record",
    )
}

pub(super) fn remove_central_recovery(heads_directory: &Path, name: &str) -> Result<(), HeadError> {
    let path = central_recovery_path(heads_directory, name)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(HeadError::FileSystem {
            action: "remove central Head recovery record",
            path,
            source,
        }),
    }
}

pub(super) fn read_manifest(
    repository: &Repository,
    worktree: &Path,
) -> Result<Option<RecoveredHead>, HeadError> {
    let path = git::worktree_private_file(repository, worktree, RECOVERY_FILE_NAME)?;
    read_recovery_record(&path, "Head recovery manifest")
}

fn serialize_recovery_record(name: &str, metadata: &HeadMetadata) -> Result<Vec<u8>, HeadError> {
    let record = RecoveryRecord {
        version: RECOVERY_VERSION,
        name: name.to_owned(),
        metadata: metadata.clone(),
    };
    let mut contents = serde_json::to_vec_pretty(&record).map_err(HeadError::SerializeState)?;
    contents.push(b'\n');
    Ok(contents)
}

fn read_recovery_record(
    path: &Path,
    kind: &'static str,
) -> Result<Option<RecoveredHead>, HeadError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(HeadError::FileSystem {
                action: "inspect Head recovery record",
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if !metadata.is_file() {
        return Err(HeadError::UnsafeProjectFile(path.to_path_buf()));
    }
    let contents = fs::read(path).map_err(|source| HeadError::FileSystem {
        action: "read Head recovery record",
        path: path.to_path_buf(),
        source,
    })?;
    let record: RecoveryRecord =
        serde_json::from_slice(&contents).map_err(|source| HeadError::InvalidLocalMetadata {
            kind,
            path: path.to_path_buf(),
            source,
        })?;
    if record.version != RECOVERY_VERSION {
        return Err(HeadError::UnsupportedLocalMetadataVersion {
            kind,
            version: record.version,
        });
    }
    Ok(Some(RecoveredHead {
        name: record.name,
        metadata: record.metadata,
    }))
}

fn central_recovery_path(heads_directory: &Path, name: &str) -> Result<PathBuf, HeadError> {
    validate_head_name(name)?;
    Ok(heads_directory
        .join(".hydra")
        .join(format!("{CENTRAL_RECOVERY_PREFIX}{name}.json")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn central_recovery_paths_reject_names_outside_the_hydra_grammar() {
        assert!(central_recovery_path(Path::new("/managed"), "nested/name").is_err());
    }
}

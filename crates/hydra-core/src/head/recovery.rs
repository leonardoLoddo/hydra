use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use super::{
    HeadError,
    git::{self, Repository},
    persistence::create_file_atomically,
    state::HeadMetadata,
};

const RECOVERY_FILE_NAME: &str = "hydra-head.json";
const RECOVERY_VERSION: u32 = 1;

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

pub(super) fn create_manifest(
    repository: &Repository,
    worktree: &Path,
    name: &str,
    metadata: &HeadMetadata,
) -> Result<(), HeadError> {
    let path = git::worktree_private_file(repository, worktree, RECOVERY_FILE_NAME)?;
    let record = RecoveryRecord {
        version: RECOVERY_VERSION,
        name: name.to_owned(),
        metadata: metadata.clone(),
    };
    let mut contents = serde_json::to_vec_pretty(&record).map_err(HeadError::SerializeState)?;
    contents.push(b'\n');
    create_file_atomically(&path, &contents)
}

pub(super) fn read_manifest(
    repository: &Repository,
    worktree: &Path,
) -> Result<Option<RecoveredHead>, HeadError> {
    let path = git::worktree_private_file(repository, worktree, RECOVERY_FILE_NAME)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(HeadError::FileSystem {
                action: "inspect Head recovery manifest",
                path,
                source,
            });
        }
    };
    if !metadata.is_file() {
        return Err(HeadError::UnsafeProjectFile(path));
    }
    let contents = fs::read(&path).map_err(|source| HeadError::FileSystem {
        action: "read Head recovery manifest",
        path: path.clone(),
        source,
    })?;
    let record: RecoveryRecord =
        serde_json::from_slice(&contents).map_err(|source| HeadError::InvalidLocalMetadata {
            kind: "Head recovery manifest",
            path: path.clone(),
            source,
        })?;
    if record.version != RECOVERY_VERSION {
        return Err(HeadError::UnsupportedLocalMetadataVersion {
            kind: "Head recovery manifest",
            version: record.version,
        });
    }
    Ok(Some(RecoveredHead {
        name: record.name,
        metadata: record.metadata,
    }))
}

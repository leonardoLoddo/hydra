mod configuration;
mod installation;

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use super::{
    HeadError,
    git::Repository,
    persistence::{StateLock, replace_state_atomically},
};
use crate::StorageBackend;
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub(super) use configuration::OpenCommandConfiguration;
use configuration::ProjectConfiguration;

const SUPPORTED_LOCAL_METADATA_VERSION: u32 = 1;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LocalState {
    version: u32,
    heads: BTreeMap<String, HeadMetadata>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct HeadMetadata {
    worktree_path: String,
    head_ref: String,
    base_ref: String,
    base_commit: String,
    target_ref: String,
    materialization_backend: String,
    created_at: String,
}

impl HeadMetadata {
    pub(super) fn new(
        path: &Path,
        branch: &str,
        base_ref: &str,
        base_commit: &str,
        target_ref: &str,
        backend: StorageBackend,
    ) -> Result<Self, HeadError> {
        let worktree_path = path
            .to_str()
            .ok_or_else(|| HeadError::UnsafeHeadsDirectory(path.to_path_buf()))?
            .to_owned();
        let materialization_backend = match backend {
            StorageBackend::CopyOnWrite => "cow",
            StorageBackend::FullCopy => "copy",
        }
        .to_owned();
        let created_at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(HeadError::Timestamp)?;

        Ok(Self {
            worktree_path,
            head_ref: format!("refs/heads/{branch}"),
            base_ref: base_ref.to_owned(),
            base_commit: base_commit.to_owned(),
            target_ref: target_ref.to_owned(),
            materialization_backend,
            created_at,
        })
    }

    pub(super) fn worktree_path(&self) -> &str {
        &self.worktree_path
    }

    pub(super) fn head_ref(&self) -> &str {
        &self.head_ref
    }

    pub(super) fn base_ref(&self) -> &str {
        &self.base_ref
    }

    pub(super) fn base_commit(&self) -> &str {
        &self.base_commit
    }

    pub(super) fn target_ref(&self) -> &str {
        &self.target_ref
    }

    pub(super) fn materialization_backend(&self) -> &str {
        &self.materialization_backend
    }

    pub(super) fn created_at(&self) -> &str {
        &self.created_at
    }
}

pub(super) struct StateSnapshot {
    configuration: ProjectConfiguration,
    state: LocalState,
    state_path: PathBuf,
}

impl StateSnapshot {
    pub(super) fn load(repository: &Repository) -> Result<Self, HeadError> {
        let configuration = ProjectConfiguration::load(&repository.root)?;
        let state_path = installation::inventory_path(&configuration, repository)?;
        let state = read_local_state(&state_path)?;
        Ok(Self {
            configuration,
            state,
            state_path,
        })
    }

    pub(super) fn heads(&self) -> &BTreeMap<String, HeadMetadata> {
        &self.state.heads
    }

    pub(super) fn head(&self, name: &str) -> Result<&HeadMetadata, HeadError> {
        self.state
            .heads
            .get(name)
            .ok_or_else(|| HeadError::HeadNotFound(name.to_owned()))
    }

    pub(super) fn heads_directory(&self) -> Result<PathBuf, HeadError> {
        heads_directory_from_state_path(&self.state_path)
    }

    pub(super) fn branch_prefix(&self) -> &str {
        self.configuration.branch_prefix()
    }

    pub(super) fn open_command(&self) -> Option<&OpenCommandConfiguration> {
        self.configuration.open_command()
    }
}

pub(super) struct StateTransaction {
    configuration: ProjectConfiguration,
    state: LocalState,
    state_path: PathBuf,
    original_state: Vec<u8>,
    lock: StateLock,
}

impl StateTransaction {
    pub(super) fn open(repository: &Repository) -> Result<Self, HeadError> {
        let configuration = ProjectConfiguration::load(&repository.root)?;
        let state_path = installation::inventory_path(&configuration, repository)?;
        let lock = StateLock::acquire(&state_path)?;
        let loaded_state = read_local_state_bytes(&state_path);
        let (original_state, state) = match loaded_state {
            Ok(loaded_state) => loaded_state,
            Err(original) => {
                return Err(match lock.release() {
                    Ok(()) => original,
                    Err(cleanup) => HeadError::RollbackFailed {
                        original: Box::new(original),
                        failures: vec![cleanup.to_string()],
                    },
                });
            }
        };

        Ok(Self {
            configuration,
            state,
            state_path,
            original_state,
            lock,
        })
    }

    pub(super) fn contains_head(&self, name: &str) -> bool {
        self.state.heads.contains_key(name)
    }

    pub(super) fn head(&self, name: &str) -> Result<&HeadMetadata, HeadError> {
        self.state
            .heads
            .get(name)
            .ok_or_else(|| HeadError::HeadNotFound(name.to_owned()))
    }

    pub(super) fn heads(&self) -> &BTreeMap<String, HeadMetadata> {
        &self.state.heads
    }

    pub(super) fn branch_prefix(&self) -> &str {
        self.configuration.branch_prefix()
    }

    pub(super) fn overlay_rules(&self) -> &[String] {
        self.configuration.overlay_rules()
    }

    pub(super) fn exclude_unsafe_overlay_symlinks(
        &mut self,
        paths: &[PathBuf],
    ) -> Result<(), HeadError> {
        self.configuration.exclude_unsafe_overlay_symlinks(paths)
    }

    pub(super) fn heads_directory(&self) -> Result<PathBuf, HeadError> {
        heads_directory_from_state_path(&self.state_path)
    }

    pub(super) fn commit(mut self, name: String, metadata: HeadMetadata) -> Result<(), HeadError> {
        self.state.heads.insert(name, metadata);
        let result = serde_json::to_vec_pretty(&self.state)
            .map_err(HeadError::SerializeState)
            .and_then(|mut bytes| {
                bytes.push(b'\n');
                replace_state_atomically(&self.state_path, &self.original_state, &bytes)
            });
        self.finish_commit(result)
    }

    pub(super) fn remove(mut self, name: &str) -> Result<(), HeadError> {
        self.state
            .heads
            .remove(name)
            .ok_or_else(|| HeadError::HeadNotFound(name.to_owned()))?;
        let result = serde_json::to_vec_pretty(&self.state)
            .map_err(HeadError::SerializeState)
            .and_then(|mut bytes| {
                bytes.push(b'\n');
                replace_state_atomically(&self.state_path, &self.original_state, &bytes)
            });
        self.finish_commit(result)
    }

    pub(super) fn remove_many(mut self, names: &[String]) -> Result<(), HeadError> {
        for name in names {
            self.state
                .heads
                .remove(name)
                .ok_or_else(|| HeadError::HeadNotFound(name.clone()))?;
        }
        let result = serde_json::to_vec_pretty(&self.state)
            .map_err(HeadError::SerializeState)
            .and_then(|mut bytes| {
                bytes.push(b'\n');
                replace_state_atomically(&self.state_path, &self.original_state, &bytes)
            });
        self.finish_commit(result)
    }

    pub(super) fn release(self) -> Result<(), HeadError> {
        self.lock.release()
    }

    pub(super) fn abort(self, original: HeadError) -> HeadError {
        match self.lock.release() {
            Ok(()) => original,
            Err(cleanup) => HeadError::RollbackFailed {
                original: Box::new(original),
                failures: vec![cleanup.to_string()],
            },
        }
    }

    fn finish_commit(self, result: Result<(), HeadError>) -> Result<(), HeadError> {
        let release = self.lock.release();
        match (result, release) {
            (Ok(()), Ok(())) => Ok(()),
            (Ok(()), Err(cleanup)) => Err(HeadError::HeadCommittedWithCleanupFailure(Box::new(
                cleanup,
            ))),
            (Err(error), Ok(())) => Err(error),
            (Err(HeadError::HeadCommittedWithCleanupFailure(original)), Err(cleanup)) => Err(
                HeadError::HeadCommittedWithCleanupFailure(Box::new(HeadError::RollbackFailed {
                    original,
                    failures: vec![cleanup.to_string()],
                })),
            ),
            (Err(original), Err(cleanup)) => Err(HeadError::RollbackFailed {
                original: Box::new(original),
                failures: vec![cleanup.to_string()],
            }),
        }
    }
}

fn read_local_state(path: &Path) -> Result<LocalState, HeadError> {
    read_local_state_bytes(path).map(|(_, state)| state)
}

fn read_local_state_bytes(path: &Path) -> Result<(Vec<u8>, LocalState), HeadError> {
    let original_state = fs::read(path).map_err(|source| HeadError::FileSystem {
        action: "read local Hydra state",
        path: path.to_path_buf(),
        source,
    })?;
    let state: LocalState =
        serde_json::from_slice(&original_state).map_err(|source| HeadError::InvalidState {
            path: path.to_path_buf(),
            source,
        })?;
    if state.version != SUPPORTED_LOCAL_METADATA_VERSION {
        return Err(HeadError::UnsupportedStateVersion(state.version));
    }
    Ok((original_state, state))
}

fn heads_directory_from_state_path(state_path: &Path) -> Result<PathBuf, HeadError> {
    state_path
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| HeadError::UnsafeHeadsDirectory(state_path.to_path_buf()))
}

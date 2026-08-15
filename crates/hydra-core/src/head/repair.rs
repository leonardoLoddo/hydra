use std::path::{Path, PathBuf};

mod application;
mod planning;

pub use application::{
    apply_abandoned_state_lock_recovery, apply_inventory_recovery, apply_pending_creation_recovery,
    apply_repairs, apply_untracked_head_recovery,
};

use super::{
    HeadError,
    state::{RepairStateSnapshot, StateSnapshot, discover_project_repository},
};
use planning::{attach_state_lock_issue, build_missing_inventory_plan, build_plan};

#[derive(Debug)]
pub struct RepairPlan {
    pub issues: Vec<RepairIssue>,
    pub stale_inventory: Vec<String>,
    pub moved_worktrees: Vec<String>,
    pub missing_inventory: Option<PathBuf>,
    pub recoverable_inventory: Vec<String>,
    pub abandoned_state_lock: Option<PathBuf>,
    pub recoverable_untracked_heads: Vec<String>,
    pub recoverable_pending_creations: Vec<String>,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum RepairIssue {
    MissingInventory {
        path: PathBuf,
    },
    RecoverableHead {
        name: String,
        path: PathBuf,
        head_ref: String,
    },
    AbandonedStateLock {
        path: PathBuf,
    },
    ActiveStateLock {
        path: PathBuf,
    },
    RecoverableUntrackedHead {
        name: String,
        path: PathBuf,
        head_ref: String,
    },
    IncompleteHeadCreation {
        name: String,
        path: PathBuf,
        head_ref: String,
    },
    StaleInventory {
        name: String,
        path: PathBuf,
        head_ref: String,
    },
    MovedHeadWorktree {
        name: String,
        recorded_path: PathBuf,
        registered_path: PathBuf,
    },
    UnregisteredHeadDirectory {
        name: String,
        path: PathBuf,
    },
    InvalidHeadDirectory {
        name: String,
        path: PathBuf,
    },
    MissingRegisteredWorktree {
        name: String,
        path: PathBuf,
        head_ref: String,
    },
    MissingHeadBranch {
        name: String,
        head_ref: String,
    },
    WorktreeBranchMismatch {
        name: String,
        path: PathBuf,
        expected_ref: String,
        observed_ref: Option<String>,
    },
    MetadataBranchMismatch {
        name: String,
        recorded_ref: String,
        expected_ref: String,
    },
    AmbiguousHeadWorktrees {
        name: String,
        head_ref: String,
        paths: Vec<PathBuf>,
    },
    UntrackedHydraWorktree {
        name: String,
        path: PathBuf,
        head_ref: String,
    },
}

#[derive(Debug)]
pub struct RepairResult {
    pub removed_stale_inventory: Vec<String>,
    pub restored_worktrees: Vec<String>,
}

#[derive(Debug)]
pub struct InventoryRecoveryResult {
    pub recovered_heads: Vec<String>,
}

#[derive(Debug)]
pub struct PendingCreationRecoveryResult {
    pub cleaned_creations: Vec<String>,
}

/// Compares Hydra inventory with Git worktree and branch state without mutation.
///
/// # Errors
///
/// Returns [`HeadError`] when repository discovery, installation validation,
/// inventory parsing, or Git worktree discovery fails.
pub fn plan_repairs(source_path: &Path) -> Result<RepairPlan, HeadError> {
    let repository = discover_project_repository(source_path)?;
    match StateSnapshot::load_for_repair(&repository)? {
        RepairStateSnapshot::Present(snapshot) => {
            let mut plan = build_plan(
                &repository,
                &snapshot.heads_directory()?,
                snapshot.branch_prefix(),
                snapshot.heads(),
            )?;
            attach_state_lock_issue(&mut plan, snapshot.state_path())?;
            Ok(plan)
        }
        RepairStateSnapshot::Missing(snapshot) => {
            let mut plan = build_missing_inventory_plan(&repository, &snapshot)?;
            attach_state_lock_issue(&mut plan, snapshot.state_path())?;
            Ok(plan)
        }
    }
}

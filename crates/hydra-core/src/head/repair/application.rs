use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use super::super::{
    HeadError, git, persistence, recovery,
    state::{
        MissingStateTransaction, RepairStateSnapshot, StateSnapshot, StateTransaction,
        discover_project_repository,
    },
};
use super::{
    InventoryRecoveryResult, PendingCreationRecoveryResult, RepairIssue, RepairResult,
    planning::{build_missing_inventory_state, build_plan, build_present_repair_state},
};

/// Rebuilds a missing inventory from explicitly approved, verified recovery manifests.
///
/// The inventory remains absent when the approved set no longer matches the
/// complete deterministic recovery plan after acquiring the state lock.
///
/// # Errors
///
/// Returns [`HeadError`] when installation validation, locking, recovery
/// manifest validation, Git inspection, or atomic publication fails.
pub fn apply_inventory_recovery(
    source_path: &Path,
    approved_heads: &[String],
) -> Result<InventoryRecoveryResult, HeadError> {
    let repository = discover_project_repository(source_path)?;
    let transaction = MissingStateTransaction::open(&repository)?;
    let heads_directory = match transaction.heads_directory() {
        Ok(path) => path,
        Err(error) => return Err(transaction.abort(error)),
    };
    let recovery = build_missing_inventory_state(
        &repository,
        transaction.state_path(),
        &heads_directory,
        transaction.branch_prefix(),
    );
    let (plan, recovered) = match recovery {
        Ok(recovery) => recovery,
        Err(error) => return Err(transaction.abort(error)),
    };
    let approved: BTreeSet<&str> = approved_heads.iter().map(String::as_str).collect();
    let current: BTreeSet<&str> = plan
        .recoverable_inventory
        .iter()
        .map(String::as_str)
        .collect();
    if current.is_empty() || approved != current {
        transaction.release()?;
        return Ok(InventoryRecoveryResult {
            recovered_heads: Vec::new(),
        });
    }

    let recovered_heads = plan.recoverable_inventory;
    transaction.commit(recovered)?;
    Ok(InventoryRecoveryResult { recovered_heads })
}

/// Removes a recognized abandoned state lock after rechecking OS ownership.
///
/// An active, malformed, unsupported, or concurrently changed lock is never
/// removed.
///
/// # Errors
///
/// Returns [`HeadError`] when installation validation, state inspection,
/// guard acquisition, lock removal, or guard cleanup fails.
pub fn apply_abandoned_state_lock_recovery(source_path: &Path) -> Result<bool, HeadError> {
    let repository = discover_project_repository(source_path)?;
    let state_path = match StateSnapshot::load_for_repair(&repository)? {
        RepairStateSnapshot::Present(snapshot) => snapshot.state_path().to_path_buf(),
        RepairStateSnapshot::Missing(snapshot) => snapshot.state_path().to_path_buf(),
    };
    persistence::remove_abandoned_state_lock(&state_path)
}

/// Adds explicitly approved manifest-backed worktrees to an existing inventory.
///
/// The inventory remains unchanged when the complete approved set no longer
/// matches the deterministic recovery candidates after locking.
///
/// # Errors
///
/// Returns [`HeadError`] when installation validation, locking, manifest or
/// Git validation, or atomic inventory publication fails.
pub fn apply_untracked_head_recovery(
    source_path: &Path,
    approved_heads: &[String],
) -> Result<InventoryRecoveryResult, HeadError> {
    let repository = discover_project_repository(source_path)?;
    let transaction = StateTransaction::open(&repository)?;
    let heads_directory = match transaction.heads_directory() {
        Ok(path) => path,
        Err(error) => return Err(transaction.abort(error)),
    };
    let recovery = build_present_repair_state(
        &repository,
        &heads_directory,
        transaction.branch_prefix(),
        transaction.heads(),
    );
    let (plan, recovered) = match recovery {
        Ok(recovery) => recovery,
        Err(error) => return Err(transaction.abort(error)),
    };
    let approved: BTreeSet<&str> = approved_heads.iter().map(String::as_str).collect();
    let current: BTreeSet<&str> = plan
        .recoverable_untracked_heads
        .iter()
        .map(String::as_str)
        .collect();
    if current.is_empty() || approved != current {
        transaction.release()?;
        return Ok(InventoryRecoveryResult {
            recovered_heads: Vec::new(),
        });
    }

    let recovered_heads = plan.recoverable_untracked_heads;
    transaction.commit_many(recovered)?;
    Ok(InventoryRecoveryResult { recovered_heads })
}

/// Cleans explicitly approved pre-worktree creation records after revalidation.
///
/// A private branch is removed only when it still points to the recorded base
/// commit and no matching worktree or managed path exists.
///
/// # Errors
///
/// Returns [`HeadError`] when installation validation, locking, Git
/// comparison, compare-and-swap branch deletion, or journal cleanup fails.
pub fn apply_pending_creation_recovery(
    source_path: &Path,
    approved_creations: &[String],
) -> Result<PendingCreationRecoveryResult, HeadError> {
    let repository = discover_project_repository(source_path)?;
    let transaction = StateTransaction::open(&repository)?;
    let heads_directory = match transaction.heads_directory() {
        Ok(path) => path,
        Err(error) => return Err(transaction.abort(error)),
    };
    let recovery = build_present_repair_state(
        &repository,
        &heads_directory,
        transaction.branch_prefix(),
        transaction.heads(),
    );
    let (plan, _) = match recovery {
        Ok(recovery) => recovery,
        Err(error) => return Err(transaction.abort(error)),
    };
    let approved: BTreeSet<&str> = approved_creations.iter().map(String::as_str).collect();
    let current: BTreeSet<&str> = plan
        .recoverable_pending_creations
        .iter()
        .map(String::as_str)
        .collect();
    if current.is_empty() || approved != current {
        transaction.release()?;
        return Ok(PendingCreationRecoveryResult {
            cleaned_creations: Vec::new(),
        });
    }

    let cleanup = (|| {
        let pending_by_name = recovery::read_pending_creations(&heads_directory)?
            .into_iter()
            .map(|pending| (pending.name().to_owned(), pending))
            .collect::<BTreeMap<_, _>>();
        for name in &plan.recoverable_pending_creations {
            let pending = pending_by_name.get(name).ok_or_else(|| {
                HeadError::ConcurrentStateChange(pending_journal_path(&heads_directory, name))
            })?;
            if !transaction.contains_head(name)
                && git::ref_exists(&repository, pending.intent().head_ref())?
            {
                git::delete_ref_if_at(
                    &repository,
                    pending.intent().head_ref(),
                    pending.intent().base_commit(),
                )?;
            }
            recovery::remove_pending_creation(pending.path())?;
        }
        Ok::<_, HeadError>(plan.recoverable_pending_creations)
    })();
    let cleaned_creations = match cleanup {
        Ok(cleaned) => cleaned,
        Err(error) => return Err(transaction.abort(error)),
    };
    transaction.release()?;
    Ok(PendingCreationRecoveryResult { cleaned_creations })
}

fn pending_journal_path(heads_directory: &Path, name: &str) -> PathBuf {
    heads_directory
        .join(".hydra")
        .join(format!("pending-{name}.json"))
}

/// Removes explicitly approved inventory entries that are still provably stale.
///
/// The private branch is never deleted. Entries that changed after planning are
/// skipped rather than being removed from a newly consistent Head.
///
/// # Errors
///
/// Returns [`HeadError`] when the current project state cannot be validated,
/// locked, inspected, or published atomically.
pub fn apply_repairs(
    source_path: &Path,
    approved_stale_inventory: &[String],
    approved_moved_worktrees: &[String],
) -> Result<RepairResult, HeadError> {
    let repository = discover_project_repository(source_path)?;
    let transaction = StateTransaction::open(&repository)?;
    let plan = match build_plan(
        &repository,
        &transaction.heads_directory()?,
        transaction.branch_prefix(),
        transaction.heads(),
    ) {
        Ok(plan) => plan,
        Err(error) => return Err(transaction.abort(error)),
    };
    let approved: BTreeSet<&str> = approved_stale_inventory
        .iter()
        .map(String::as_str)
        .collect();
    let approved_moved: BTreeSet<&str> = approved_moved_worktrees
        .iter()
        .map(String::as_str)
        .collect();
    let approved_current_moved: BTreeSet<&str> = plan
        .moved_worktrees
        .iter()
        .filter_map(|name| {
            approved_moved
                .contains(name.as_str())
                .then_some(name.as_str())
        })
        .collect();
    let mut restored_worktrees = Vec::new();
    for issue in &plan.issues {
        let RepairIssue::MovedHeadWorktree {
            name,
            recorded_path,
            registered_path,
        } = issue
        else {
            continue;
        };
        if approved_current_moved.contains(name.as_str()) {
            if let Err(error) =
                git::move_registered_worktree(&repository, registered_path, recorded_path)
            {
                return Err(transaction.abort(error));
            }
            restored_worktrees.push(name.clone());
        }
    }
    let removed_stale_inventory: Vec<String> = plan
        .stale_inventory
        .into_iter()
        .filter(|name| approved.contains(name.as_str()))
        .collect();

    if removed_stale_inventory.is_empty() {
        transaction.release()?;
    } else {
        transaction.remove_many(&removed_stale_inventory)?;
    }

    Ok(RepairResult {
        removed_stale_inventory,
        restored_worktrees,
    })
}

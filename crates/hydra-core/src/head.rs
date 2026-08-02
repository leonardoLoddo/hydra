mod close;
mod error;
mod git;
mod inspection;
mod materializer;
mod open;
mod overlay;
mod persistence;
mod recovery;
mod removal;
mod repair;
mod state;

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
};

pub use close::{CloseOutcome, ClosedHead, IntegrationResult, IntegrationStrategy, close_head};
pub use error::HeadError;
use git::{Repository, TrackedEntry};
pub use inspection::{
    ChangeCounts, HeadInspection, HeadSummary, ProjectInspection, WorktreeHead, head_path,
    inspect_head, inspect_project, list_heads,
};
use materializer::materialize_tracked_files;
pub use open::{OpenedHead, open_head};
use overlay::{OverlayPlan, materialize_overlays, plan_overlays};
pub use removal::{RemoveHeadOptions, RemovedHead, remove_head};
pub use repair::{
    InventoryRecoveryResult, RepairIssue, RepairPlan, RepairResult,
    apply_abandoned_state_lock_recovery, apply_inventory_recovery, apply_repairs, plan_repairs,
};
use state::{HeadMetadata, StateSnapshot, StateTransaction, discover_project_repository};

use crate::StorageBackend;

pub(crate) fn validated_heads_directory(source_path: &Path) -> Result<PathBuf, HeadError> {
    let repository = discover_project_repository(source_path)?;
    StateSnapshot::load(&repository)?.heads_directory()
}

#[derive(Debug)]
pub struct CreateHeadOptions {
    pub name: String,
    pub from: Option<String>,
    pub target: Option<String>,
    pub confirmed_full_copy: bool,
    pub exclude_unsafe_overlay_symlinks: bool,
}

#[derive(Debug)]
pub struct CreatedHead {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub storage_backend: StorageBackend,
    pub overlay_files: usize,
    pub overlay_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum HeadCreationProgress {
    PlanningOverlays,
    MaterializingTrackedEntries { entries: usize },
    MaterializingOverlayEntries { entries: usize },
}

struct ProgressReporter<Observer> {
    observer: Observer,
    enabled: bool,
}

impl<Observer: FnMut(HeadCreationProgress)> ProgressReporter<Observer> {
    fn new(observer: Observer) -> Self {
        Self {
            observer,
            enabled: true,
        }
    }

    fn report(&mut self, progress: HeadCreationProgress) {
        if self.enabled && catch_unwind(AssertUnwindSafe(|| (self.observer)(progress))).is_err() {
            self.enabled = false;
        }
    }
}

/// Creates an isolated Git worktree and records it as a Hydra Head.
///
/// # Errors
///
/// Returns [`HeadError`] when the project is not initialized, the requested
/// refs or destinations conflict, Git fails, materialization fails, or Hydra
/// cannot persist a consistent state.
pub fn create_head(
    source_path: &Path,
    options: CreateHeadOptions,
) -> Result<CreatedHead, HeadError> {
    create_head_with_progress(source_path, options, |_| {})
}

/// Creates an isolated Git worktree and reports coarse-grained progress.
///
/// # Errors
///
/// Returns [`HeadError`] under the same conditions as [`create_head`].
pub fn create_head_with_progress(
    source_path: &Path,
    options: CreateHeadOptions,
    report_progress: impl FnMut(HeadCreationProgress),
) -> Result<CreatedHead, HeadError> {
    validate_head_name(&options.name)?;

    let repository = discover_project_repository(source_path)?;
    let mut transaction = StateTransaction::open(&repository)?;
    let mut report_progress = ProgressReporter::new(report_progress);

    let prepared = match prepare_head(
        &repository,
        &mut transaction,
        &options,
        &mut report_progress,
    ) {
        Ok(prepared) => prepared,
        Err(error) => return Err(transaction.abort(error)),
    };

    if let Err(error) = git::create_branch(&repository, &prepared.branch, &prepared.base_commit) {
        return Err(transaction.abort(error));
    }
    let creation = create_worktree(
        &repository,
        &prepared,
        options.confirmed_full_copy,
        &mut report_progress,
    );
    let storage_backend = match creation {
        Ok(backend) => backend,
        Err(error) => {
            let error =
                rollback_worktree(&repository, &prepared.head_path, &prepared.branch, error);
            return Err(transaction.abort(error));
        }
    };

    let metadata = match HeadMetadata::new(
        &prepared.head_path,
        &prepared.branch,
        &prepared.base_ref,
        &prepared.base_commit,
        &prepared.target_ref,
        storage_backend,
    ) {
        Ok(metadata) => metadata,
        Err(error) => {
            let error =
                rollback_worktree(&repository, &prepared.head_path, &prepared.branch, error);
            return Err(transaction.abort(error));
        }
    };
    if let Err(error) =
        recovery::create_manifest(&repository, &prepared.head_path, &options.name, &metadata)
    {
        let error = rollback_worktree(&repository, &prepared.head_path, &prepared.branch, error);
        return Err(transaction.abort(error));
    }
    if let Err(error) = transaction.commit(options.name.clone(), metadata) {
        if error.head_was_committed() {
            return Err(error);
        }
        return Err(rollback_worktree(
            &repository,
            &prepared.head_path,
            &prepared.branch,
            error,
        ));
    }

    Ok(CreatedHead {
        name: options.name,
        path: prepared.head_path,
        branch: prepared.branch,
        storage_backend,
        overlay_files: prepared.overlay_plan.file_count(),
        overlay_bytes: prepared.overlay_plan.total_bytes(),
    })
}

struct PreparedHead {
    heads_directory: PathBuf,
    head_path: PathBuf,
    branch: String,
    base_commit: String,
    base_ref: String,
    target_ref: String,
    tracked_entries: Vec<TrackedEntry>,
    overlay_plan: OverlayPlan,
}

fn prepare_head(
    repository: &Repository,
    transaction: &mut StateTransaction,
    options: &CreateHeadOptions,
    report_progress: &mut ProgressReporter<impl FnMut(HeadCreationProgress)>,
) -> Result<PreparedHead, HeadError> {
    if transaction.contains_head(&options.name) {
        return Err(HeadError::HeadAlreadyExists(options.name.clone()));
    }

    let heads_directory = transaction.heads_directory()?;
    let head_path = heads_directory.join(&options.name);
    ensure_destination_absent(&head_path)?;

    let branch = format!("{}{}", transaction.branch_prefix(), options.name);
    git::validate_branch_name(repository, &branch)?;
    git::ensure_branch_absent(repository, &branch)?;

    let requested_base = options.from.as_deref().unwrap_or("HEAD");
    let base_commit = git::resolve_commit(repository, requested_base)?;
    let base_ref = git::normalize_ref(
        repository,
        requested_base,
        "normalizing the base ref",
        "base ref",
    )?;
    let target_ref = resolve_target_ref(repository, options.target.as_deref(), &base_ref)?;
    let tracked_entries = git::tracked_entries(repository, &base_commit)?;
    report_progress.report(HeadCreationProgress::PlanningOverlays);
    let overlay_plan = plan_overlays(
        &repository.root,
        &heads_directory,
        transaction.overlay_rules(),
        &tracked_entries,
    );
    let overlay_plan = match overlay_plan {
        Err(HeadError::UnsafeOverlaySymlinks { paths })
            if options.exclude_unsafe_overlay_symlinks =>
        {
            transaction.exclude_unsafe_overlay_symlinks(&paths)?;
            plan_overlays(
                &repository.root,
                &heads_directory,
                transaction.overlay_rules(),
                &tracked_entries,
            )?
        }
        result => result?,
    };
    if overlay_plan.full_copy_file_count() > 0 && !options.confirmed_full_copy {
        return Err(HeadError::OverlayFullCopyConfirmationRequired {
            files: overlay_plan.full_copy_file_count(),
            bytes: overlay_plan.full_copy_bytes(),
        });
    }

    Ok(PreparedHead {
        heads_directory,
        head_path,
        branch,
        base_commit,
        base_ref,
        target_ref,
        tracked_entries,
        overlay_plan,
    })
}

fn create_worktree(
    repository: &Repository,
    prepared: &PreparedHead,
    confirmed_full_copy: bool,
    report_progress: &mut ProgressReporter<impl FnMut(HeadCreationProgress)>,
) -> Result<StorageBackend, HeadError> {
    let reuse_tracked_sources = git::worktree_matches_commit(repository, &prepared.base_commit)?;
    git::add_worktree(repository, &prepared.head_path, &prepared.branch)?;
    git::initialize_index(&prepared.head_path, &prepared.base_commit)?;
    if !prepared.tracked_entries.is_empty() {
        report_progress.report(HeadCreationProgress::MaterializingTrackedEntries {
            entries: prepared.tracked_entries.len(),
        });
    }
    let mut backend = materialize_tracked_files(
        repository,
        &prepared.heads_directory,
        &prepared.head_path,
        &prepared.tracked_entries,
        reuse_tracked_sources,
    )?;
    if prepared.overlay_plan.file_count() > 0 {
        report_progress.report(HeadCreationProgress::MaterializingOverlayEntries {
            entries: prepared.overlay_plan.file_count(),
        });
    }
    if materialize_overlays(
        repository,
        &prepared.overlay_plan,
        &prepared.head_path,
        confirmed_full_copy,
    )? == StorageBackend::FullCopy
    {
        backend = StorageBackend::FullCopy;
    }
    git::verify_clean_worktree(&prepared.head_path)?;
    Ok(backend)
}

fn rollback_worktree(
    repository: &Repository,
    head_path: &Path,
    branch: &str,
    original: HeadError,
) -> HeadError {
    let mut failures = Vec::new();
    if let Err(error) = git::remove_worktree(repository, head_path) {
        failures.push(error.to_string());
    }
    if let Err(error) = git::delete_branch(repository, branch) {
        failures.push(error.to_string());
    }

    if failures.is_empty() {
        original
    } else {
        HeadError::RollbackFailed {
            original: Box::new(original),
            failures,
        }
    }
}

fn resolve_target_ref(
    repository: &Repository,
    requested_target: Option<&str>,
    base_ref: &str,
) -> Result<String, HeadError> {
    if let Some(target) = requested_target {
        return git::resolve_local_branch(repository, target);
    }
    if base_ref.starts_with("refs/heads/") {
        Ok(base_ref.to_owned())
    } else {
        Err(HeadError::TargetRequired)
    }
}

pub(super) fn validate_head_name(name: &str) -> Result<(), HeadError> {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return Err(HeadError::InvalidName(name.to_owned()));
    };
    if !first.is_ascii_alphanumeric()
        || !characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
        || Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lock"))
        || name.contains("..")
    {
        return Err(HeadError::InvalidName(name.to_owned()));
    }
    Ok(())
}

fn ensure_destination_absent(path: &Path) -> Result<(), HeadError> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Err(HeadError::DestinationExists(path.to_path_buf())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(HeadError::FileSystem {
            action: "inspect Head destination",
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::{HeadCreationProgress, ProgressReporter};

    #[test]
    fn a_panicking_progress_observer_is_disabled_after_the_first_panic() {
        let calls = Cell::new(0);
        let mut reporter = ProgressReporter::new(|_| {
            calls.set(calls.get() + 1);
            panic!("progress observers must not interrupt Head creation");
        });

        reporter.report(HeadCreationProgress::PlanningOverlays);
        reporter.report(HeadCreationProgress::PlanningOverlays);

        assert_eq!(calls.get(), 1);
    }
}

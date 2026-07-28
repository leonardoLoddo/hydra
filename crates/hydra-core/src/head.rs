mod error;
mod git;
mod materializer;
mod overlay;
mod persistence;
mod state;

use std::path::{Path, PathBuf};

pub use error::HeadError;
use git::Repository;
use materializer::materialize_tracked_files;
use overlay::{OverlayPlan, materialize_overlays, plan_overlays};
use state::{HeadMetadata, StateTransaction};

use crate::StorageBackend;

#[derive(Debug)]
pub struct CreateHeadOptions {
    pub name: String,
    pub from: Option<String>,
    pub target: Option<String>,
    pub confirmed_overlays: bool,
}

#[derive(Debug)]
pub struct CreatedHead {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub storage_backend: StorageBackend,
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
    validate_head_name(&options.name)?;

    let repository = Repository::discover(source_path)?;
    let transaction = StateTransaction::open(&repository)?;

    let prepared = match prepare_head(&repository, &transaction, &options) {
        Ok(prepared) => prepared,
        Err(error) => return Err(transaction.abort(error)),
    };

    if let Err(error) = git::create_branch(&repository, &prepared.branch, &prepared.base_commit) {
        return Err(transaction.abort(error));
    }
    let creation = create_worktree(
        &repository,
        &prepared.heads_directory,
        &prepared.head_path,
        &prepared.branch,
        &prepared.base_commit,
        &prepared.overlay_plan,
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
    })
}

struct PreparedHead {
    heads_directory: PathBuf,
    head_path: PathBuf,
    branch: String,
    base_commit: String,
    base_ref: String,
    target_ref: String,
    overlay_plan: OverlayPlan,
}

fn prepare_head(
    repository: &Repository,
    transaction: &StateTransaction,
    options: &CreateHeadOptions,
) -> Result<PreparedHead, HeadError> {
    if transaction.contains_head(&options.name) {
        return Err(HeadError::HeadAlreadyExists(options.name.clone()));
    }

    let heads_directory = transaction.heads_directory(repository)?;
    let head_path = heads_directory.join(&options.name);
    ensure_destination_absent(&head_path)?;

    let branch = format!("{}{}", transaction.branch_prefix(), options.name);
    git::validate_branch_name(repository, &branch)?;
    git::ensure_branch_absent(repository, &branch)?;

    let requested_base = options.from.as_deref().unwrap_or("HEAD");
    let base_commit = git::resolve_commit(repository, requested_base)?;
    let base_ref = git::normalize_ref(repository, requested_base)?;
    let target_ref = resolve_target_ref(repository, options.target.as_deref(), &base_ref)?;
    let tracked_entries = git::tracked_entries(repository, &base_commit)?;
    let overlay_plan = plan_overlays(
        &repository.root,
        transaction.overlay_rules(),
        &tracked_entries,
    )?;
    if !overlay_plan.is_empty() && !options.confirmed_overlays {
        return Err(HeadError::OverlayConfirmationRequired {
            files: overlay_plan.file_count(),
            bytes: overlay_plan.total_bytes(),
        });
    }

    Ok(PreparedHead {
        heads_directory,
        head_path,
        branch,
        base_commit,
        base_ref,
        target_ref,
        overlay_plan,
    })
}

fn create_worktree(
    repository: &Repository,
    heads_directory: &Path,
    head_path: &Path,
    branch: &str,
    base_commit: &str,
    overlay_plan: &OverlayPlan,
) -> Result<StorageBackend, HeadError> {
    git::add_worktree(repository, head_path, branch)?;
    git::initialize_index(head_path, base_commit)?;
    let mut backend =
        materialize_tracked_files(repository, heads_directory, head_path, base_commit)?;
    if materialize_overlays(repository, overlay_plan, head_path)? == StorageBackend::FullCopy {
        backend = StorageBackend::FullCopy;
    }
    git::verify_clean_worktree(head_path)?;
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

fn validate_head_name(name: &str) -> Result<(), HeadError> {
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

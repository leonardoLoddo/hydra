use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{
    HeadError,
    git::{self, Repository},
    recovery,
    state::{StateTransaction, discover_project_repository},
    validate_head_name,
};

#[derive(Debug)]
pub struct RemoveHeadOptions {
    pub name: String,
    pub force: bool,
}

#[derive(Debug)]
pub struct RemovedHead {
    pub name: String,
    pub preserved_branch: Option<String>,
}

/// Removes a validated local Head and its integrated private branch.
///
/// # Errors
///
/// Returns [`HeadError`] when the installation or recorded Head is
/// inconsistent, the worktree has uncommitted changes without `force`, the
/// private branch is not integrated without `force`, or Git/state removal
/// cannot be completed safely.
pub fn remove_head(
    source_path: &Path,
    options: RemoveHeadOptions,
) -> Result<RemovedHead, HeadError> {
    validate_head_name(&options.name)?;
    let repository = discover_project_repository(source_path)?;
    let transaction = StateTransaction::open(&repository)?;
    let prepared = match prepare_removal(&repository, &transaction, &options) {
        Ok(prepared) => prepared,
        Err(error) => return Err(transaction.abort(error)),
    };

    if let Err(error) = git::remove_registered_worktree(&repository, &prepared.path, options.force)
    {
        return Err(transaction.abort(error));
    }

    if let Err(source) = transaction.remove(&options.name, || {
        recovery::remove_central_recovery(&prepared.heads_directory, &options.name)
    }) {
        return Err(HeadError::HeadRemovalIncomplete {
            name: options.name,
            preserved_branch: prepared.head_ref,
            source: Box::new(source),
        });
    }

    let integrated_now = git::ref_exists(&repository, &prepared.target_ref)?
        && git::is_ancestor(&repository, &prepared.head_commit, &prepared.target_ref)?;
    let preserved_branch = if integrated_now {
        if let Err(source) =
            git::delete_ref_if_matches(&repository, &prepared.head_ref, &prepared.head_commit)
        {
            return Err(HeadError::HeadRemovalIncomplete {
                name: options.name,
                preserved_branch: prepared.head_ref,
                source: Box::new(source),
            });
        }
        None
    } else {
        Some(prepared.head_ref)
    };

    Ok(RemovedHead {
        name: options.name,
        preserved_branch,
    })
}

struct PreparedRemoval {
    heads_directory: PathBuf,
    path: PathBuf,
    head_ref: String,
    head_commit: String,
    target_ref: String,
}

fn prepare_removal(
    repository: &Repository,
    transaction: &StateTransaction,
    options: &RemoveHeadOptions,
) -> Result<PreparedRemoval, HeadError> {
    let metadata = transaction.head(&options.name)?;
    let heads_directory = transaction.heads_directory()?;
    let path = PathBuf::from(metadata.worktree_path());
    if !path.is_absolute() || path != heads_directory.join(&options.name) {
        return Err(HeadError::UnsafeHeadPath(path));
    }
    let expected_head_ref = format!("refs/heads/{}{}", transaction.branch_prefix(), options.name);
    if metadata.head_ref() != expected_head_ref {
        return Err(inconsistent(
            &options.name,
            "Head branch does not match metadata",
        ));
    }
    require_real_directory(&path, &options.name)?;
    if !git::worktree_paths(repository)?
        .iter()
        .any(|registered| registered == &path)
    {
        return Err(inconsistent(
            &options.name,
            "worktree is not registered with Git",
        ));
    }
    if git::symbolic_head(&path)?.as_deref() != Some(metadata.head_ref()) {
        return Err(inconsistent(
            &options.name,
            "worktree branch does not match metadata",
        ));
    }
    if !git::ref_exists(repository, metadata.head_ref())? {
        return Err(inconsistent(&options.name, "Head branch is missing"));
    }
    if !git::ref_exists(repository, metadata.target_ref())? {
        return Err(inconsistent(&options.name, "target ref is missing"));
    }
    let head_commit = git::commit_for_ref(repository, metadata.head_ref())?;
    if git::worktree_commit(&path)? != head_commit {
        return Err(inconsistent(
            &options.name,
            "worktree commit does not match the Head branch",
        ));
    }
    let changes = git::worktree_changes(&path)?;
    if !options.force && !changes.is_clean() {
        return Err(HeadError::HeadHasUncommittedChanges(options.name.clone()));
    }
    let integrated = git::is_ancestor(repository, &head_commit, metadata.target_ref())?;
    if !options.force && !integrated {
        return Err(HeadError::HeadHasUnintegratedCommits {
            head_ref: metadata.head_ref().to_owned(),
            target_ref: metadata.target_ref().to_owned(),
        });
    }

    Ok(PreparedRemoval {
        heads_directory,
        path,
        head_ref: metadata.head_ref().to_owned(),
        head_commit,
        target_ref: metadata.target_ref().to_owned(),
    })
}

fn require_real_directory(path: &Path, name: &str) -> Result<(), HeadError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(inconsistent(name, "worktree path is not a real directory")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(inconsistent(name, "worktree path is missing"))
        }
        Err(source) => Err(HeadError::FileSystem {
            action: "inspect Head worktree",
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn inconsistent(name: &str, reason: &'static str) -> HeadError {
    HeadError::HeadRemovalInconsistent {
        name: name.to_owned(),
        reason,
    }
}

use std::path::Path;

use super::{
    HeadError,
    git::{self, Repository},
    inspection::inspect_head,
    removal::{RemoveHeadOptions, remove_head},
    validate_head_name,
};

#[derive(Debug)]
pub struct ClosedHead {
    pub name: String,
    pub target_ref: String,
    pub target_commit: String,
}

/// Integrates a clean Head into its target and removes it safely.
///
/// # Errors
///
/// Returns [`HeadError`] when the Head is inconsistent or dirty, its target is
/// checked out in another worktree, integration conflicts or races, or the
/// protected removal cannot complete.
pub fn close_head(source_path: &Path, name: &str) -> Result<ClosedHead, HeadError> {
    validate_head_name(name)?;
    let repository = Repository::discover(source_path)?;
    let inspection = inspect_head(source_path, name)?;
    if !inspection.consistency_issues.is_empty() {
        return Err(HeadError::HeadCloseInconsistent {
            name: name.to_owned(),
            reason: inspection.consistency_issues.join(", "),
        });
    }
    let changes = inspection
        .changes
        .as_ref()
        .ok_or_else(|| HeadError::HeadCloseInconsistent {
            name: name.to_owned(),
            reason: "worktree status is unavailable".to_owned(),
        })?;
    if changes.modified > 0 || changes.added > 0 || changes.deleted > 0 || changes.untracked > 0 {
        return Err(HeadError::HeadHasUncommittedChanges(name.to_owned()));
    }
    ensure_target_not_checked_out(&repository, &inspection.target_ref)?;

    let head_commit = inspection
        .commit
        .ok_or_else(|| HeadError::HeadCloseInconsistent {
            name: name.to_owned(),
            reason: "worktree commit is unavailable".to_owned(),
        })?;
    let target_before = git::commit_for_ref(&repository, &inspection.target_ref)?;
    let target_commit = if git::is_ancestor(&repository, &head_commit, &target_before)? {
        target_before
    } else if git::is_ancestor(&repository, &target_before, &head_commit)? {
        git::update_ref_if_matches(
            &repository,
            &inspection.target_ref,
            &head_commit,
            &target_before,
        )?;
        head_commit.clone()
    } else {
        let tree =
            git::merge_tree(&repository, &target_before, &head_commit).map_err(
                |error| match error {
                    HeadError::GitCommandFailed {
                        status: Some(1), ..
                    } => HeadError::HeadCloseConflict {
                        name: name.to_owned(),
                        target_ref: inspection.target_ref.clone(),
                    },
                    other => other,
                },
            )?;
        let merge_commit = git::create_merge_commit(
            &repository,
            &tree,
            &target_before,
            &head_commit,
            &format!(
                "Merge {} into {}",
                inspection.recorded_head_ref, inspection.target_ref
            ),
        )?;
        git::update_ref_if_matches(
            &repository,
            &inspection.target_ref,
            &merge_commit,
            &target_before,
        )?;
        merge_commit
    };

    if let Err(source) = remove_head(
        source_path,
        RemoveHeadOptions {
            name: name.to_owned(),
            force: false,
        },
    ) {
        return Err(HeadError::HeadIntegratedButRemovalFailed {
            name: name.to_owned(),
            target_ref: inspection.target_ref,
            target_commit,
            source: Box::new(source),
        });
    }

    Ok(ClosedHead {
        name: name.to_owned(),
        target_ref: inspection.target_ref,
        target_commit,
    })
}

fn ensure_target_not_checked_out(
    repository: &Repository,
    target_ref: &str,
) -> Result<(), HeadError> {
    for path in git::worktree_paths(repository)? {
        if git::symbolic_head(&path)?.as_deref() == Some(target_ref) {
            return Err(HeadError::HeadCloseTargetCheckedOut {
                target_ref: target_ref.to_owned(),
                path,
            });
        }
    }
    Ok(())
}

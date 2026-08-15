use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};

use super::{
    HeadError,
    command_template::{self, CommandTemplateError},
    git::{self, Repository},
    inspection::inspect_head,
    removal::{RemoveHeadOptions, remove_head},
    state::{CloseCommandConfiguration, StateSnapshot, discover_project_repository},
    validate_head_name,
};

#[derive(Debug)]
pub struct ClosedHead {
    pub name: String,
    pub target_ref: String,
    pub outcome: CloseOutcome,
}

#[derive(Debug)]
pub enum CloseOutcome {
    Integrated {
        target_commit: String,
        strategy: IntegrationStrategy,
        result: IntegrationResult,
    },
    CommandCompleted {
        target_before: String,
        target_after: Option<String>,
        removed: bool,
    },
}

#[derive(Debug)]
pub enum IntegrationStrategy {
    CheckoutFree,
    TargetWorktree { path: PathBuf },
}

#[derive(Debug)]
pub enum IntegrationResult {
    AlreadyIntegrated,
    FastForward,
    MergeCommit,
}

/// Runs the configured close adapter, or integrates and removes a clean Head.
///
/// # Errors
///
/// Returns [`HeadError`] when the Head is inconsistent or dirty, a configured
/// adapter is invalid or fails, native integration conflicts or races, or
/// protected removal cannot complete.
pub fn close_head(source_path: &Path, name: &str) -> Result<ClosedHead, HeadError> {
    validate_head_name(name)?;
    let repository = discover_project_repository(source_path)?;
    let snapshot = StateSnapshot::load(&repository)?;
    if let Some(command) = snapshot.close_command().cloned() {
        return close_with_command(source_path, name, &repository, &command);
    }
    close_with_merge(source_path, name, &repository)
}

fn close_with_merge(
    source_path: &Path,
    name: &str,
    repository: &Repository,
) -> Result<ClosedHead, HeadError> {
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
        return Err(HeadError::HeadCloseHasUncommittedChanges(name.to_owned()));
    }
    let head_commit = inspection
        .commit
        .ok_or_else(|| HeadError::HeadCloseInconsistent {
            name: name.to_owned(),
            reason: "worktree commit is unavailable".to_owned(),
        })?;
    let target_before = git::commit_for_ref(repository, &inspection.target_ref)?;
    let target_worktree = checked_out_target(repository, &inspection.target_ref)?;
    let removal_source =
        stable_control_worktree(repository, &inspection.path, target_worktree.as_deref())?;
    let (target_commit, strategy, result) = match target_worktree {
        Some(path) => integrate_in_target_worktree(
            repository,
            name,
            &inspection.recorded_head_ref,
            &inspection.target_ref,
            &target_before,
            &head_commit,
            path,
        )?,
        None => integrate_checkout_free(
            repository,
            name,
            &inspection.recorded_head_ref,
            &inspection.target_ref,
            &target_before,
            &head_commit,
        )?,
    };

    if let Err(source) = remove_head(
        &removal_source,
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
        outcome: CloseOutcome::Integrated {
            target_commit,
            strategy,
            result,
        },
    })
}

#[allow(clippy::too_many_arguments)]
fn integrate_in_target_worktree(
    repository: &Repository,
    name: &str,
    head_ref: &str,
    target_ref: &str,
    target_before: &str,
    head_commit: &str,
    path: PathBuf,
) -> Result<(String, IntegrationStrategy, IntegrationResult), HeadError> {
    verify_target_worktree(&path, target_ref, target_before)?;
    let (target_commit, result) = if git::is_ancestor(repository, head_commit, target_before)? {
        (
            target_before.to_owned(),
            IntegrationResult::AlreadyIntegrated,
        )
    } else if git::is_ancestor(repository, target_before, head_commit)? {
        verify_target_worktree(&path, target_ref, target_before)?;
        git::fast_forward_worktree(&path, head_commit)?;
        verify_target_worktree(&path, target_ref, head_commit)?;
        (head_commit.to_owned(), IntegrationResult::FastForward)
    } else {
        let merge_commit = prepare_merge_commit(
            repository,
            name,
            head_ref,
            target_ref,
            target_before,
            head_commit,
        )?;
        verify_target_worktree(&path, target_ref, target_before)?;
        git::fast_forward_worktree(&path, &merge_commit)?;
        verify_target_worktree(&path, target_ref, &merge_commit)?;
        (merge_commit, IntegrationResult::MergeCommit)
    };
    Ok((
        target_commit,
        IntegrationStrategy::TargetWorktree { path },
        result,
    ))
}

fn integrate_checkout_free(
    repository: &Repository,
    name: &str,
    head_ref: &str,
    target_ref: &str,
    target_before: &str,
    head_commit: &str,
) -> Result<(String, IntegrationStrategy, IntegrationResult), HeadError> {
    let (target_commit, result) = if git::is_ancestor(repository, head_commit, target_before)? {
        (
            target_before.to_owned(),
            IntegrationResult::AlreadyIntegrated,
        )
    } else if git::is_ancestor(repository, target_before, head_commit)? {
        git::update_ref_if_matches(repository, target_ref, head_commit, target_before)?;
        (head_commit.to_owned(), IntegrationResult::FastForward)
    } else {
        let merge_commit = prepare_merge_commit(
            repository,
            name,
            head_ref,
            target_ref,
            target_before,
            head_commit,
        )?;
        git::update_ref_if_matches(repository, target_ref, &merge_commit, target_before)?;
        (merge_commit, IntegrationResult::MergeCommit)
    };
    Ok((target_commit, IntegrationStrategy::CheckoutFree, result))
}

fn prepare_merge_commit(
    repository: &Repository,
    name: &str,
    head_ref: &str,
    target_ref: &str,
    target_commit: &str,
    head_commit: &str,
) -> Result<String, HeadError> {
    let tree =
        git::merge_tree(repository, target_commit, head_commit).map_err(|error| match error {
            HeadError::GitCommandFailed {
                status: Some(1), ..
            } => HeadError::HeadCloseConflict {
                name: name.to_owned(),
                target_ref: target_ref.to_owned(),
            },
            other => other,
        })?;
    git::create_merge_commit(
        repository,
        &tree,
        target_commit,
        head_commit,
        &format!("Merge {head_ref} into {target_ref}"),
    )
}

fn close_with_command(
    source_path: &Path,
    name: &str,
    repository: &Repository,
    command: &CloseCommandConfiguration,
) -> Result<ClosedHead, HeadError> {
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
        return Err(HeadError::HeadCloseHasUncommittedChanges(name.to_owned()));
    }
    let removal_source = stable_control_worktree(repository, &inspection.path, None)?;
    let target_before = git::commit_for_ref(repository, &inspection.target_ref)?;
    let path = inspection
        .path
        .to_str()
        .ok_or_else(|| HeadError::HeadCloseInconsistent {
            name: name.to_owned(),
            reason: "worktree path is not valid UTF-8".to_owned(),
        })?;
    let placeholders = BTreeMap::from([
        ("{name}", name),
        ("{path}", path),
        ("{headRef}", inspection.recorded_head_ref.as_str()),
        ("{baseRef}", inspection.base_ref.as_str()),
        ("{targetRef}", inspection.target_ref.as_str()),
    ]);
    let (program_template, argument_templates, remove_on_success) = command.command();
    let program = expand(program_template, &placeholders)?;
    if program.is_empty() {
        return Err(HeadError::InvalidCloseCommand(
            "program must not be empty".to_owned(),
        ));
    }
    let args = argument_templates
        .iter()
        .map(|argument| expand(argument, &placeholders))
        .collect::<Result<Vec<_>, _>>()?;
    let status = Command::new(&program)
        .args(args)
        .current_dir(&inspection.path)
        .status()
        .map_err(|source| HeadError::CloseCommandUnavailable {
            program: program.clone(),
            source,
        })?;
    let target_after = target_commit_if_present(repository, &inspection.target_ref)?;
    if !status.success() {
        return Err(HeadError::CloseCommandFailed {
            program,
            status: status.code(),
            target_ref: inspection.target_ref,
            target_before,
            target_after,
        });
    }
    if remove_on_success
        && let Err(source) = remove_head(
            &removal_source,
            RemoveHeadOptions {
                name: name.to_owned(),
                force: false,
            },
        )
    {
        return Err(HeadError::HeadCloseCommandCompletedButRemovalFailed {
            name: name.to_owned(),
            target_ref: inspection.target_ref,
            target_before,
            target_after,
            source: Box::new(source),
        });
    }

    Ok(ClosedHead {
        name: name.to_owned(),
        target_ref: inspection.target_ref,
        outcome: CloseOutcome::CommandCompleted {
            target_before,
            target_after,
            removed: remove_on_success,
        },
    })
}

fn target_commit_if_present(
    repository: &Repository,
    target_ref: &str,
) -> Result<Option<String>, HeadError> {
    if git::ref_exists(repository, target_ref)? {
        git::commit_for_ref(repository, target_ref).map(Some)
    } else {
        Ok(None)
    }
}

fn expand(
    template: &str,
    placeholders: &BTreeMap<&'static str, &str>,
) -> Result<String, HeadError> {
    command_template::expand(template, placeholders).map_err(|error| match error {
        CommandTemplateError::UnsupportedPlaceholder => unsupported_placeholder(template),
        CommandTemplateError::Nul => {
            HeadError::InvalidCloseCommand("program and arguments must not contain NUL".to_owned())
        }
    })
}

fn unsupported_placeholder(template: &str) -> HeadError {
    HeadError::InvalidCloseCommand(format!("unsupported placeholder in {template:?}"))
}

fn checked_out_target(
    repository: &Repository,
    target_ref: &str,
) -> Result<Option<PathBuf>, HeadError> {
    let mut target_worktree = None;
    for worktree in git::registered_worktrees(repository)? {
        if worktree.branch.as_deref() == Some(target_ref) {
            if target_worktree.is_some() {
                return Err(HeadError::HeadCloseInconsistent {
                    name: target_ref.to_owned(),
                    reason: "target branch is checked out in multiple worktrees".to_owned(),
                });
            }
            target_worktree = Some(worktree.path);
        }
    }
    Ok(target_worktree)
}

fn stable_control_worktree(
    repository: &Repository,
    closing_head_path: &Path,
    target_worktree: Option<&Path>,
) -> Result<PathBuf, HeadError> {
    if repository.root != closing_head_path {
        return Ok(repository.root.clone());
    }
    if let Some(path) = target_worktree {
        return Ok(path.to_path_buf());
    }
    git::worktree_paths(repository)?
        .into_iter()
        .find(|path| path != closing_head_path)
        .ok_or_else(|| HeadError::HeadCloseInconsistent {
            name: closing_head_path.display().to_string(),
            reason: "no stable worktree remains available to complete removal".to_owned(),
        })
}

fn verify_target_worktree(
    path: &Path,
    target_ref: &str,
    expected_commit: &str,
) -> Result<(), HeadError> {
    let observed_ref = git::symbolic_head(path)?;
    if observed_ref.as_deref() != Some(target_ref) {
        return Err(HeadError::HeadCloseInconsistent {
            name: target_ref.to_owned(),
            reason: "target worktree branch changed during close".to_owned(),
        });
    }
    if git::worktree_commit(path)? != expected_commit {
        return Err(HeadError::HeadCloseInconsistent {
            name: target_ref.to_owned(),
            reason: "target worktree commit changed during close".to_owned(),
        });
    }
    if let Some(operation) = git::worktree_operation(path)? {
        return Err(HeadError::HeadCloseTargetWorktreeOperation {
            target_ref: target_ref.to_owned(),
            path: path.to_path_buf(),
            operation,
        });
    }
    let changes = git::worktree_changes(path)?;
    if !changes.is_clean() {
        return Err(HeadError::HeadCloseTargetWorktreeDirty {
            target_ref: target_ref.to_owned(),
            path: path.to_path_buf(),
            modified: changes.modified,
            added: changes.added,
            deleted: changes.deleted,
            untracked: changes.untracked,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::expand;

    #[test]
    fn close_placeholder_values_may_contain_literal_braces() {
        let placeholders = BTreeMap::from([("{path}", "/projects/{demo}/payment")]);

        let expanded =
            expand("--folder={path}", &placeholders).expect("value braces should remain literal");

        assert_eq!(expanded, "--folder=/projects/{demo}/payment");
    }

    #[test]
    fn unsupported_close_template_placeholders_are_rejected() {
        let placeholders = BTreeMap::from([("{path}", "/projects/demo/payment")]);

        let error = expand("{unknown}", &placeholders)
            .expect_err("unknown placeholders must not reach the adapter");

        assert!(error.to_string().contains("unsupported placeholder"));
    }
}

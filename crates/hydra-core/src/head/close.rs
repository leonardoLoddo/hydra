use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
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
    TargetWorktree { path: PathBuf },
}

#[derive(Debug)]
pub enum IntegrationResult {
    AlreadyIntegrated,
    FastForward,
    MergeCommit,
}

#[derive(Clone, Debug)]
pub enum HeadCloseProgress {
    WaitingForMergeResolution { path: PathBuf },
}

/// From the parent project, runs its close adapter or integrates and removes a
/// clean Head.
///
/// # Errors
///
/// Returns [`HeadError`] when invoked outside the parent project worktree, the
/// Head or native target is not ready, a configured adapter is invalid or
/// fails, a resolved merge is inconsistent, or protected removal cannot
/// complete.
pub fn close_head(source_path: &Path, name: &str) -> Result<ClosedHead, HeadError> {
    close_head_with_progress(source_path, name, |_| {})
}

/// Closes a Head and reports when a native Git merge needs manual resolution.
///
/// # Errors
///
/// Returns the same errors as [`close_head`].
pub fn close_head_with_progress(
    source_path: &Path,
    name: &str,
    mut report_progress: impl FnMut(HeadCloseProgress),
) -> Result<ClosedHead, HeadError> {
    validate_head_name(name)?;
    let invocation = Repository::discover(source_path)?;
    let repository = discover_project_repository(source_path)?;
    if invocation.root != repository.root {
        return Err(HeadError::HeadCloseRequiresParentWorktree {
            parent: repository.root,
        });
    }
    let snapshot = StateSnapshot::load(&repository)?;
    if let Some(command) = snapshot.close_command().cloned() {
        return close_with_command(source_path, name, &repository, &command);
    }
    close_with_merge(source_path, name, &repository, &mut report_progress)
}

fn close_with_merge(
    source_path: &Path,
    name: &str,
    repository: &Repository,
    report_progress: &mut impl FnMut(HeadCloseProgress),
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
    let (target_commit, strategy, result) = integrate_in_parent_worktree(
        repository,
        name,
        &inspection.target_ref,
        &target_before,
        &head_commit,
        report_progress,
    )?;

    if let Err(source) = remove_head(
        &repository.root,
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
fn integrate_in_parent_worktree(
    repository: &Repository,
    name: &str,
    target_ref: &str,
    target_before: &str,
    head_commit: &str,
    report_progress: &mut impl FnMut(HeadCloseProgress),
) -> Result<(String, IntegrationStrategy, IntegrationResult), HeadError> {
    verify_target_worktree(&repository.root, target_ref, target_before)?;
    let result = if git::is_ancestor(repository, head_commit, target_before)? {
        IntegrationResult::AlreadyIntegrated
    } else if git::is_ancestor(repository, target_before, head_commit)? {
        IntegrationResult::FastForward
    } else {
        IntegrationResult::MergeCommit
    };
    let status = git::merge_in_worktree(&repository.root, head_commit)?;
    if !status.success() {
        if git::worktree_operation(&repository.root)? == Some("merge") {
            report_progress(HeadCloseProgress::WaitingForMergeResolution {
                path: repository.root.clone(),
            });
            let target_commit = wait_for_merge_resolution(
                repository,
                name,
                target_ref,
                target_before,
                head_commit,
            )?;
            verify_target_worktree(&repository.root, target_ref, &target_commit)?;
            return Ok((
                target_commit,
                IntegrationStrategy::TargetWorktree {
                    path: repository.root.clone(),
                },
                IntegrationResult::MergeCommit,
            ));
        }
        return Err(HeadError::GitCommandFailed {
            operation: "merging the Head in the parent project worktree",
            status: status.code(),
            stderr: "Git wrote its diagnostics directly to the terminal".to_owned(),
        });
    }
    let target_commit = git::commit_for_ref(repository, target_ref)?;
    verify_target_worktree(&repository.root, target_ref, &target_commit)?;
    Ok((
        target_commit,
        IntegrationStrategy::TargetWorktree {
            path: repository.root.clone(),
        },
        result,
    ))
}

fn wait_for_merge_resolution(
    repository: &Repository,
    name: &str,
    target_ref: &str,
    target_before: &str,
    head_commit: &str,
) -> Result<String, HeadError> {
    loop {
        if let Some(operation) = git::worktree_operation(&repository.root)? {
            if operation != "merge" {
                return Err(HeadError::HeadCloseInconsistent {
                    name: name.to_owned(),
                    reason: format!("target worktree entered a {operation} operation during close"),
                });
            }
            let merge_head = match git::commit_for_ref(repository, "MERGE_HEAD") {
                Ok(merge_head) => merge_head,
                Err(_) if git::worktree_operation(&repository.root)?.is_none() => continue,
                Err(error) => return Err(error),
            };
            if merge_head != head_commit {
                return Err(HeadError::HeadCloseInconsistent {
                    name: name.to_owned(),
                    reason: "Git merge no longer targets the recorded Head commit".to_owned(),
                });
            }
            thread::sleep(Duration::from_millis(50));
            continue;
        }

        let current = git::worktree_commit(&repository.root)?;
        let changes = git::worktree_changes(&repository.root)?;
        if current == target_before && changes.is_clean() {
            return Err(HeadError::HeadCloseAborted {
                name: name.to_owned(),
            });
        }
        if !changes.is_clean() {
            return Err(HeadError::HeadCloseInconsistent {
                name: name.to_owned(),
                reason: "target worktree has changes after Git merge resolution".to_owned(),
            });
        }
        if git::commit_for_ref(repository, target_ref)? != current {
            return Err(HeadError::HeadCloseInconsistent {
                name: name.to_owned(),
                reason: "target branch and worktree diverged during close".to_owned(),
            });
        }
        if git::commit_parents(repository, &current)?
            != [target_before.to_owned(), head_commit.to_owned()]
        {
            return Err(HeadError::HeadCloseInconsistent {
                name: name.to_owned(),
                reason: "resolved merge commit does not have the recorded target and Head parents"
                    .to_owned(),
            });
        }
        return Ok(current);
    }
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
    let removal_source = repository.root.clone();
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

fn verify_target_worktree(
    path: &Path,
    target_ref: &str,
    expected_commit: &str,
) -> Result<(), HeadError> {
    let observed_ref = git::symbolic_head(path)?;
    if observed_ref.as_deref() != Some(target_ref) {
        return Err(HeadError::HeadCloseRequiresTargetBranch {
            target_ref: target_ref.to_owned(),
            parent: path.to_path_buf(),
            current_ref: observed_ref,
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

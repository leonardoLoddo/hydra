use std::{collections::BTreeMap, path::Path, process::Command};

use super::{
    HeadError,
    git::{self, Repository},
    inspection::inspect_head,
    removal::{RemoveHeadOptions, remove_head},
    state::{CloseCommandConfiguration, StateSnapshot},
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
    },
    CommandCompleted {
        target_before: String,
        target_after: Option<String>,
        removed: bool,
    },
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
    let repository = Repository::discover(source_path)?;
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
    ensure_target_not_checked_out(repository, &inspection.target_ref)?;

    let head_commit = inspection
        .commit
        .ok_or_else(|| HeadError::HeadCloseInconsistent {
            name: name.to_owned(),
            reason: "worktree commit is unavailable".to_owned(),
        })?;
    let target_before = git::commit_for_ref(repository, &inspection.target_ref)?;
    let target_commit = if git::is_ancestor(repository, &head_commit, &target_before)? {
        target_before
    } else if git::is_ancestor(repository, &target_before, &head_commit)? {
        git::update_ref_if_matches(
            repository,
            &inspection.target_ref,
            &head_commit,
            &target_before,
        )?;
        head_commit.clone()
    } else {
        let tree =
            git::merge_tree(repository, &target_before, &head_commit).map_err(
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
            repository,
            &tree,
            &target_before,
            &head_commit,
            &format!(
                "Merge {} into {}",
                inspection.recorded_head_ref, inspection.target_ref
            ),
        )?;
        git::update_ref_if_matches(
            repository,
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
        outcome: CloseOutcome::Integrated { target_commit },
    })
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
            source_path,
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
    let mut remaining = template;
    let mut expanded = String::with_capacity(template.len());
    while let Some(open) = remaining.find('{') {
        let (literal, placeholder_and_rest) = remaining.split_at(open);
        if literal.contains('}') {
            return Err(unsupported_placeholder(template));
        }
        expanded.push_str(literal);
        let Some(close) = placeholder_and_rest.find('}') else {
            return Err(unsupported_placeholder(template));
        };
        let (placeholder, rest) = placeholder_and_rest.split_at(close + 1);
        let value = placeholders
            .get(placeholder)
            .ok_or_else(|| unsupported_placeholder(template))?;
        expanded.push_str(value);
        remaining = rest;
    }
    if remaining.contains('}') {
        return Err(unsupported_placeholder(template));
    }
    expanded.push_str(remaining);
    if expanded.contains('\0') {
        return Err(HeadError::InvalidCloseCommand(
            "program and arguments must not contain NUL".to_owned(),
        ));
    }
    Ok(expanded)
}

fn unsupported_placeholder(template: &str) -> HeadError {
    HeadError::InvalidCloseCommand(format!("unsupported placeholder in {template:?}"))
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

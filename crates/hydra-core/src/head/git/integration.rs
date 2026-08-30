use std::{
    path::Path,
    process::{Command, ExitStatus},
};

use super::{Repository, command_failure, run_git, stdout_line};
use crate::head::HeadError;

pub(in crate::head) fn is_ancestor(
    repository: &Repository,
    ancestor: &str,
    descendant: &str,
) -> Result<bool, HeadError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(&repository.root)
        .args(["merge-base", "--is-ancestor"])
        .arg(ancestor)
        .arg(descendant)
        .output()
        .map_err(HeadError::GitUnavailable)?;
    if output.status.success() {
        Ok(true)
    } else if output.status.code() == Some(1) {
        Ok(false)
    } else {
        Err(command_failure(
            "checking whether the Head is integrated",
            &output,
        ))
    }
}

pub(in crate::head) fn delete_ref_if_matches(
    repository: &Repository,
    reference: &str,
    expected_commit: &str,
) -> Result<(), HeadError> {
    run_git(
        &repository.root,
        &["update-ref", "-d", reference, expected_commit],
        "deleting the integrated Head branch",
    )
    .map(|_| ())
}

pub(in crate::head) fn merge_in_worktree(
    path: &Path,
    head_commit: &str,
) -> Result<ExitStatus, HeadError> {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["merge", "--no-edit"])
        .arg(head_commit)
        .status()
        .map_err(HeadError::GitUnavailable)
}

pub(in crate::head) fn commit_parents(
    repository: &Repository,
    commit: &str,
) -> Result<Vec<String>, HeadError> {
    let output = run_git(
        &repository.root,
        &["show", "-s", "--format=%P", commit],
        "reading integration commit parents",
    )?;
    Ok(stdout_line(&output, "integration commit parents")?
        .split_ascii_whitespace()
        .map(str::to_owned)
        .collect())
}

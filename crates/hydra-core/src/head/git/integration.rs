use std::{path::Path, process::Command};

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

pub(in crate::head) fn update_ref_if_matches(
    repository: &Repository,
    reference: &str,
    new_commit: &str,
    expected_commit: &str,
) -> Result<(), HeadError> {
    run_git(
        &repository.root,
        &["update-ref", reference, new_commit, expected_commit],
        "updating the integration target",
    )
    .map(|_| ())
}

pub(in crate::head) fn fast_forward_worktree(
    path: &Path,
    head_commit: &str,
) -> Result<(), HeadError> {
    run_git(
        path,
        &["merge", "--ff-only", "--no-edit", "--", head_commit],
        "fast-forwarding the checked-out integration target",
    )
    .map(|_| ())
}

pub(in crate::head) fn merge_tree(
    repository: &Repository,
    target_commit: &str,
    head_commit: &str,
) -> Result<String, HeadError> {
    let output = run_git(
        &repository.root,
        &["merge-tree", "--write-tree", target_commit, head_commit],
        "merging the Head without a worktree",
    )?;
    let tree = output
        .stdout
        .split(|byte| *byte == b'\n')
        .next()
        .ok_or(HeadError::InvalidGitOutput("merged tree"))?;
    std::str::from_utf8(tree)
        .ok()
        .filter(|tree| !tree.is_empty())
        .map(str::to_owned)
        .ok_or(HeadError::InvalidGitOutput("merged tree"))
}

pub(in crate::head) fn create_merge_commit(
    repository: &Repository,
    tree: &str,
    target_parent: &str,
    head_parent: &str,
    message: &str,
) -> Result<String, HeadError> {
    let output = run_git(
        &repository.root,
        &[
            "commit-tree",
            tree,
            "-p",
            target_parent,
            "-p",
            head_parent,
            "-m",
            message,
        ],
        "creating the integration commit",
    )?;
    stdout_line(&output, "integration commit")
}

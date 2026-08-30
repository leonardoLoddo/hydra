use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

use super::HeadError;

mod integration;
mod protocol;

pub(super) use integration::{
    commit_parents, delete_ref_if_matches, is_ancestor, merge_in_worktree,
};

use protocol::{bytes_to_path, parse_registered_worktrees, parse_worktree_changes};

pub(super) struct Repository {
    pub(super) root: PathBuf,
    pub(super) git_common_directory: PathBuf,
}

#[derive(Debug)]
pub(super) struct RegisteredWorktree {
    pub(super) path: PathBuf,
    pub(super) branch: Option<String>,
}

#[derive(Debug)]
pub(super) struct TrackedEntry {
    pub(super) mode: String,
    pub(super) object: String,
    pub(super) path: PathBuf,
}

#[derive(Debug, Default)]
pub(super) struct WorktreeChanges {
    pub(super) modified: usize,
    pub(super) added: usize,
    pub(super) deleted: usize,
    pub(super) untracked: usize,
}

impl WorktreeChanges {
    pub(super) fn is_clean(&self) -> bool {
        self.modified == 0 && self.added == 0 && self.deleted == 0 && self.untracked == 0
    }
}

impl Repository {
    pub(super) fn discover(path: &Path) -> Result<Self, HeadError> {
        let discovered_root = git_path(path, "--show-toplevel", "repository root")?;
        let root = crate::path::canonicalize(&discovered_root).map_err(|source| {
            HeadError::FileSystem {
                action: "resolve repository root",
                path: discovered_root,
                source,
            }
        })?;
        let discovered_common = git_path(&root, "--git-common-dir", "Git common directory")?;
        let git_common_directory =
            crate::path::canonicalize(&discovered_common).map_err(|source| {
                HeadError::FileSystem {
                    action: "resolve Git common directory",
                    path: discovered_common,
                    source,
                }
            })?;
        Ok(Self {
            root,
            git_common_directory,
        })
    }
}

pub(super) fn worktree_paths(repository: &Repository) -> Result<Vec<PathBuf>, HeadError> {
    registered_worktrees(repository).map(|worktrees| {
        worktrees
            .into_iter()
            .map(|worktree| worktree.path)
            .collect()
    })
}

pub(super) fn worktree_private_file(
    repository: &Repository,
    worktree: &Path,
    file_name: &str,
) -> Result<PathBuf, HeadError> {
    let output = run_git(
        worktree,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            file_name,
        ],
        "locating private Head metadata",
    )?;
    let candidate = bytes_to_path(&stdout_record(&output, "private Head metadata path")?)?;
    if candidate.file_name().and_then(|name| name.to_str()) != Some(file_name) {
        return Err(HeadError::UnsafeProjectFile(candidate));
    }
    let parent = candidate
        .parent()
        .ok_or_else(|| HeadError::UnsafeProjectFile(candidate.clone()))?;
    let parent = crate::path::canonicalize(parent).map_err(|source| HeadError::FileSystem {
        action: "resolve private Head metadata directory",
        path: parent.to_path_buf(),
        source,
    })?;
    let common = crate::path::canonicalize(&repository.git_common_directory).map_err(|source| {
        HeadError::FileSystem {
            action: "resolve Git common directory",
            path: repository.git_common_directory.clone(),
            source,
        }
    })?;
    let worktrees = common.join("worktrees");
    if !parent.starts_with(&worktrees) || parent == worktrees {
        return Err(HeadError::UnsafeProjectFile(candidate));
    }
    Ok(parent.join(file_name))
}

pub(super) fn registered_worktrees(
    repository: &Repository,
) -> Result<Vec<RegisteredWorktree>, HeadError> {
    let output = run_git(
        &repository.root,
        &["worktree", "list", "--porcelain", "-z"],
        "listing Git worktrees",
    )?;
    parse_registered_worktrees(&output.stdout)
}

pub(super) fn ref_exists(repository: &Repository, reference: &str) -> Result<bool, HeadError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(&repository.root)
        .args(["show-ref", "--verify", "--quiet"])
        .arg("--")
        .arg(reference)
        .output()
        .map_err(HeadError::GitUnavailable)?;
    if output.status.success() {
        Ok(true)
    } else if output.status.code() == Some(1) {
        Ok(false)
    } else {
        Err(command_failure("checking the Head ref", &output))
    }
}

pub(super) fn commit_for_ref(
    repository: &Repository,
    reference: &str,
) -> Result<String, HeadError> {
    let revision = format!("{reference}^{{commit}}");
    let output = run_git(
        &repository.root,
        &["rev-parse", "--verify", "--end-of-options", &revision],
        "resolving the Head commit",
    )?;
    stdout_line(&output, "Head commit")
}

pub(super) fn ahead_behind(
    repository: &Repository,
    base_commit: &str,
    head_commit: &str,
) -> Result<(usize, usize), HeadError> {
    let range = format!("{base_commit}...{head_commit}");
    let output = run_git(
        &repository.root,
        &["rev-list", "--left-right", "--count", &range],
        "calculating Head ahead/behind",
    )?;
    let value = stdout_line(&output, "ahead/behind counts")?;
    let mut counts = value.split_ascii_whitespace();
    let behind = counts
        .next()
        .and_then(|count| count.parse::<usize>().ok())
        .ok_or(HeadError::InvalidGitOutput("behind count"))?;
    let ahead = counts
        .next()
        .and_then(|count| count.parse::<usize>().ok())
        .ok_or(HeadError::InvalidGitOutput("ahead count"))?;
    if counts.next().is_some() {
        return Err(HeadError::InvalidGitOutput("ahead/behind counts"));
    }
    Ok((ahead, behind))
}

pub(super) fn worktree_changes(path: &Path) -> Result<WorktreeChanges, HeadError> {
    let output = run_git(
        path,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        "reading Head status",
    )?;
    parse_worktree_changes(&output.stdout)
}

pub(super) fn symbolic_head(path: &Path) -> Result<Option<String>, HeadError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["symbolic-ref", "--quiet", "HEAD"])
        .output()
        .map_err(HeadError::GitUnavailable)?;
    if output.status.success() {
        stdout_line(&output, "Head branch").map(Some)
    } else if output.status.code() == Some(1) {
        Ok(None)
    } else {
        Err(command_failure("reading the Head branch", &output))
    }
}

pub(super) fn worktree_commit(path: &Path) -> Result<String, HeadError> {
    let output = run_git(
        path,
        &["rev-parse", "--verify", "HEAD^{commit}"],
        "reading the worktree commit",
    )?;
    stdout_line(&output, "worktree commit")
}

pub(super) fn worktree_operation(path: &Path) -> Result<Option<&'static str>, HeadError> {
    const OPERATIONS: [(&str, &str); 8] = [
        ("MERGE_HEAD", "merge"),
        ("CHERRY_PICK_HEAD", "cherry-pick"),
        ("REVERT_HEAD", "revert"),
        ("rebase-merge", "rebase"),
        ("rebase-apply", "rebase"),
        ("BISECT_START", "bisect"),
        ("BISECT_LOG", "bisect"),
        ("sequencer", "sequenced Git"),
    ];
    for (marker, operation) in OPERATIONS {
        let output = run_git(
            path,
            &["rev-parse", "--path-format=absolute", "--git-path", marker],
            "locating Git operation state",
        )?;
        let marker_path = bytes_to_path(&stdout_record(&output, "Git operation path")?)?;
        if marker_path
            .try_exists()
            .map_err(|source| HeadError::FileSystem {
                action: "inspect Git operation state",
                path: marker_path,
                source,
            })?
        {
            return Ok(Some(operation));
        }
    }
    Ok(None)
}

pub(super) fn resolve_commit(
    repository: &Repository,
    reference: &str,
) -> Result<String, HeadError> {
    let revision = format!("{reference}^{{commit}}");
    let output = run_git(
        &repository.root,
        &["rev-parse", "--verify", "--end-of-options", &revision],
        "resolving the base commit",
    )?;
    stdout_line(&output, "base commit")
}

pub(super) fn normalize_ref(
    repository: &Repository,
    reference: &str,
    operation: &'static str,
    output_name: &'static str,
) -> Result<String, HeadError> {
    if reference.starts_with('-') {
        return Err(HeadError::InvalidRef(reference.to_owned()));
    }
    let output = run_git(
        &repository.root,
        &["rev-parse", "--symbolic-full-name", reference],
        operation,
    )?;
    let value = output.stdout.strip_suffix(b"\n").unwrap_or(&output.stdout);
    let value = value.strip_suffix(b"\r").unwrap_or(value);
    if value.is_empty() {
        Ok(reference.to_owned())
    } else {
        String::from_utf8(value.to_vec()).map_err(|_| HeadError::InvalidGitOutput(output_name))
    }
}

pub(super) fn resolve_local_branch(
    repository: &Repository,
    reference: &str,
) -> Result<String, HeadError> {
    let normalized = normalize_ref(
        repository,
        reference,
        "normalizing the target ref",
        "target ref",
    )?;
    if normalized.starts_with("refs/heads/") {
        resolve_commit(repository, &normalized)?;
        Ok(normalized)
    } else {
        Err(HeadError::InvalidRef(reference.to_owned()))
    }
}

pub(super) fn validate_branch_name(repository: &Repository, branch: &str) -> Result<(), HeadError> {
    run_git(
        &repository.root,
        &["check-ref-format", "--branch", branch],
        "validating the Head branch",
    )
    .map(|_| ())
}

pub(super) fn ensure_branch_absent(repository: &Repository, branch: &str) -> Result<(), HeadError> {
    let reference = format!("refs/heads/{branch}");
    let output = Command::new("git")
        .arg("-C")
        .arg(&repository.root)
        .args(["show-ref", "--verify", "--quiet"])
        .arg(&reference)
        .output()
        .map_err(HeadError::GitUnavailable)?;
    if output.status.success() {
        Err(HeadError::BranchAlreadyExists(branch.to_owned()))
    } else if output.status.code() == Some(1) {
        Ok(())
    } else {
        Err(command_failure("checking the Head branch", &output))
    }
}

pub(super) fn create_branch(
    repository: &Repository,
    branch: &str,
    base_commit: &str,
) -> Result<(), HeadError> {
    run_git(
        &repository.root,
        &["branch", "--", branch, base_commit],
        "creating the Head branch",
    )
    .map(|_| ())
}

pub(super) fn add_worktree(
    repository: &Repository,
    path: &Path,
    branch: &str,
) -> Result<(), HeadError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(&repository.root)
        .args(["worktree", "add", "--no-checkout", "--"])
        .arg(path)
        .arg(branch)
        .output()
        .map_err(HeadError::GitUnavailable)?;
    ensure_success("creating the Git worktree", &output)
}

pub(super) fn initialize_index(path: &Path, base_commit: &str) -> Result<(), HeadError> {
    run_git(
        path,
        &["read-tree", base_commit],
        "initializing the Head index",
    )
    .map(|_| ())
}

pub(super) fn tracked_entries(
    repository: &Repository,
    base_commit: &str,
) -> Result<Vec<TrackedEntry>, HeadError> {
    let output = run_git(
        &repository.root,
        &["ls-tree", "-r", "-z", "--full-tree", base_commit],
        "listing tracked files",
    )?;
    let mut entries = Vec::new();
    for record in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|r| !r.is_empty())
    {
        let Some(tab) = record.iter().position(|byte| *byte == b'\t') else {
            return Err(HeadError::InvalidGitOutput("tree entry"));
        };
        let header = std::str::from_utf8(&record[..tab])
            .map_err(|_| HeadError::InvalidGitOutput("tree entry header"))?;
        let mut fields = header.split_ascii_whitespace();
        let mode = fields
            .next()
            .ok_or(HeadError::InvalidGitOutput("tree mode"))?;
        let kind = fields
            .next()
            .ok_or(HeadError::InvalidGitOutput("tree type"))?;
        let object = fields
            .next()
            .ok_or(HeadError::InvalidGitOutput("tree object"))?;
        if fields.next().is_some() || !matches!(kind, "blob" | "commit") {
            return Err(HeadError::InvalidGitOutput("tree entry"));
        }
        entries.push(TrackedEntry {
            mode: mode.to_owned(),
            object: object.to_owned(),
            path: bytes_to_path(&record[tab + 1..])?,
        });
    }
    Ok(entries)
}

pub(super) fn worktree_matches_commit(
    repository: &Repository,
    commit: &str,
) -> Result<bool, HeadError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(&repository.root)
        .args(["diff", "--quiet", "--no-ext-diff"])
        .arg(commit)
        .arg("--")
        .output()
        .map_err(HeadError::GitUnavailable)?;
    if output.status.success() {
        Ok(true)
    } else if output.status.code() == Some(1) {
        Ok(false)
    } else {
        Err(command_failure(
            "checking reusable tracked working files",
            &output,
        ))
    }
}

pub(super) fn verify_clean_worktree(path: &Path) -> Result<(), HeadError> {
    let output = run_git(path, &["status", "--porcelain"], "verifying the new Head")?;
    if output.stdout.is_empty() {
        Ok(())
    } else {
        Err(HeadError::InvalidGitOutput("clean Head status"))
    }
}

pub(super) fn remove_worktree(repository: &Repository, path: &Path) -> Result<(), HeadError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(&repository.root)
        .args(["worktree", "remove", "--force"])
        .arg(path)
        .output()
        .map_err(HeadError::GitUnavailable)?;
    if output.status.success() || output.status.code() == Some(128) && !path.exists() {
        Ok(())
    } else {
        Err(command_failure("removing the incomplete worktree", &output))
    }
}

pub(super) fn remove_registered_worktree(
    repository: &Repository,
    path: &Path,
    force: bool,
) -> Result<(), HeadError> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(&repository.root)
        .args(["worktree", "remove"]);
    if force {
        command.arg("--force");
    }
    let output = command
        .arg(path)
        .output()
        .map_err(HeadError::GitUnavailable)?;
    ensure_success("removing the Head worktree", &output)
}

pub(super) fn move_registered_worktree(
    repository: &Repository,
    source: &Path,
    destination: &Path,
) -> Result<(), HeadError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(&repository.root)
        .args(["worktree", "move", "--"])
        .arg(source)
        .arg(destination)
        .output()
        .map_err(HeadError::GitUnavailable)?;
    ensure_success("restoring the Head worktree path", &output)
}

pub(super) fn delete_branch(repository: &Repository, branch: &str) -> Result<(), HeadError> {
    let output = run_git(
        &repository.root,
        &["branch", "-D", branch],
        "removing the incomplete Head branch",
    );
    match output {
        Ok(_)
        | Err(HeadError::GitCommandFailed {
            status: Some(1), ..
        }) => Ok(()),
        Err(error) => Err(error),
    }
}

pub(super) fn delete_ref_if_at(
    repository: &Repository,
    reference: &str,
    expected_commit: &str,
) -> Result<(), HeadError> {
    run_git(
        &repository.root,
        &["update-ref", "-d", reference, expected_commit],
        "removing the incomplete Head branch",
    )
    .map(|_| ())
}

fn git_path(path: &Path, argument: &str, field: &'static str) -> Result<PathBuf, HeadError> {
    let output = run_git(
        path,
        &["rev-parse", "--path-format=absolute", argument],
        "discovering the Git repository",
    )?;
    let bytes = stdout_record(&output, field)?;
    bytes_to_path(&bytes)
}

fn run_git(path: &Path, arguments: &[&str], operation: &'static str) -> Result<Output, HeadError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(arguments)
        .output()
        .map_err(HeadError::GitUnavailable)?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(command_failure(operation, &output))
    }
}

fn ensure_success(operation: &'static str, output: &Output) -> Result<(), HeadError> {
    if output.status.success() {
        Ok(())
    } else {
        Err(command_failure(operation, output))
    }
}

fn command_failure(operation: &'static str, output: &Output) -> HeadError {
    HeadError::GitCommandFailed {
        operation,
        status: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn stdout_line(output: &Output, field: &'static str) -> Result<String, HeadError> {
    let bytes = stdout_record(output, field)?;
    String::from_utf8(bytes).map_err(|_| HeadError::InvalidGitOutput(field))
}

fn stdout_record(output: &Output, field: &'static str) -> Result<Vec<u8>, HeadError> {
    let value = output.stdout.strip_suffix(b"\n").unwrap_or(&output.stdout);
    let value = value.strip_suffix(b"\r").unwrap_or(value);
    if value.is_empty() {
        Err(HeadError::InvalidGitOutput(field))
    } else {
        Ok(value.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, process::Command};

    use tempfile::tempdir;

    use super::{Repository, worktree_matches_commit};

    #[test]
    fn a_clean_worktree_can_reuse_files_from_the_requested_commit() {
        let temporary = tempdir().expect("temporary directory should be created");
        initialize_repository(temporary.path());
        let repository = test_repository(temporary.path());
        let commit = git_stdout(temporary.path(), &["rev-parse", "HEAD"]);

        assert!(
            worktree_matches_commit(&repository, &commit)
                .expect("clean worktree comparison should succeed")
        );

        fs::write(temporary.path().join("untracked"), b"ignored by comparison")
            .expect("untracked fixture should be written");
        assert!(
            worktree_matches_commit(&repository, &commit)
                .expect("untracked files should not affect tracked reuse")
        );
    }

    #[test]
    fn a_tracked_change_disables_worktree_file_reuse() {
        let temporary = tempdir().expect("temporary directory should be created");
        initialize_repository(temporary.path());
        let repository = test_repository(temporary.path());
        let commit = git_stdout(temporary.path(), &["rev-parse", "HEAD"]);
        fs::write(temporary.path().join("tracked"), b"changed\n")
            .expect("tracked fixture should be changed");

        assert!(
            !worktree_matches_commit(&repository, &commit)
                .expect("changed worktree comparison should succeed")
        );
    }

    fn initialize_repository(path: &Path) {
        run_git(path, &["init", "--quiet"]);
        fs::write(path.join("tracked"), b"base\n").expect("tracked fixture should be written");
        run_git(path, &["add", "tracked"]);
        run_git(
            path,
            &[
                "-c",
                "user.name=Hydra Tests",
                "-c",
                "user.email=hydra-tests@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
    }

    fn test_repository(path: &Path) -> Repository {
        Repository {
            root: path.to_path_buf(),
            git_common_directory: path.join(".git"),
        }
    }

    fn run_git(path: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(arguments)
            .output()
            .expect("Git should run");
        assert!(
            output.status.success(),
            "Git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_stdout(path: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(arguments)
            .output()
            .expect("Git should run");
        assert!(output.status.success());
        String::from_utf8(output.stdout)
            .expect("Git output should be UTF-8")
            .trim_end()
            .to_owned()
    }
}

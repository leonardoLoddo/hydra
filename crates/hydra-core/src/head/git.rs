use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

use super::HeadError;

pub(super) struct Repository {
    pub(super) root: PathBuf,
    pub(super) git_common_directory: PathBuf,
}

#[derive(Debug)]
pub(super) struct TrackedEntry {
    pub(super) mode: String,
    pub(super) object: String,
    pub(super) path: PathBuf,
}

impl Repository {
    pub(super) fn discover(path: &Path) -> Result<Self, HeadError> {
        let root = git_path(path, "--show-toplevel", "repository root")?;
        let git_common_directory = git_path(&root, "--git-common-dir", "Git common directory")?;
        Ok(Self {
            root,
            git_common_directory,
        })
    }
}

pub(super) fn worktree_paths(repository: &Repository) -> Result<Vec<PathBuf>, HeadError> {
    let output = run_git(
        &repository.root,
        &["worktree", "list", "--porcelain", "-z"],
        "listing Git worktrees",
    )?;
    let mut paths = Vec::new();
    for record in output.stdout.split(|byte| *byte == 0) {
        if let Some(value) = record.strip_prefix(b"worktree ") {
            if value.is_empty() {
                return Err(HeadError::InvalidGitOutput("worktree path"));
            }
            paths.push(bytes_to_path(value)?);
        }
    }
    if paths.is_empty() {
        Err(HeadError::InvalidGitOutput("worktree list"))
    } else {
        Ok(paths)
    }
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

pub(super) fn normalize_ref(repository: &Repository, reference: &str) -> Result<String, HeadError> {
    if reference.starts_with('-') {
        return Err(HeadError::InvalidRef(reference.to_owned()));
    }
    let output = run_git(
        &repository.root,
        &["rev-parse", "--symbolic-full-name", reference],
        "normalizing the base ref",
    )?;
    let value = output.stdout.strip_suffix(b"\n").unwrap_or(&output.stdout);
    let value = value.strip_suffix(b"\r").unwrap_or(value);
    if value.is_empty() {
        Ok(reference.to_owned())
    } else {
        String::from_utf8(value.to_vec()).map_err(|_| HeadError::InvalidGitOutput("base ref"))
    }
}

pub(super) fn resolve_local_branch(
    repository: &Repository,
    reference: &str,
) -> Result<String, HeadError> {
    let normalized = normalize_ref(repository, reference)?;
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

#[cfg(unix)]
#[allow(clippy::unnecessary_wraps)]
fn bytes_to_path(value: &[u8]) -> Result<PathBuf, HeadError> {
    Ok(PathBuf::from(OsString::from_vec(value.to_vec())))
}

#[cfg(not(unix))]
fn bytes_to_path(value: &[u8]) -> Result<PathBuf, HeadError> {
    let value = std::str::from_utf8(value).map_err(|_| HeadError::InvalidGitOutput("Git path"))?;
    Ok(PathBuf::from(value))
}

use std::{ffi::OsString, path::PathBuf};

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

use super::{RegisteredWorktree, WorktreeChanges};
use crate::head::HeadError;

pub(super) fn parse_registered_worktrees(
    bytes: &[u8],
) -> Result<Vec<RegisteredWorktree>, HeadError> {
    let mut worktrees = Vec::new();
    let mut current: Option<RegisteredWorktree> = None;
    for record in bytes.split(|byte| *byte == 0) {
        if let Some(value) = record.strip_prefix(b"worktree ") {
            if let Some(worktree) = current.take() {
                worktrees.push(worktree);
            }
            if value.is_empty() {
                return Err(HeadError::InvalidGitOutput("worktree path"));
            }
            current = Some(RegisteredWorktree {
                path: bytes_to_path(value)?,
                branch: None,
            });
        } else if let Some(value) = record.strip_prefix(b"branch ") {
            let worktree = current
                .as_mut()
                .ok_or(HeadError::InvalidGitOutput("worktree list"))?;
            if value.is_empty() {
                return Err(HeadError::InvalidGitOutput("worktree branch"));
            }
            worktree.branch = Some(
                String::from_utf8(value.to_vec())
                    .map_err(|_| HeadError::InvalidGitOutput("worktree branch"))?,
            );
        }
    }
    if let Some(worktree) = current {
        worktrees.push(worktree);
    }
    if worktrees.is_empty() {
        Err(HeadError::InvalidGitOutput("worktree list"))
    } else {
        Ok(worktrees)
    }
}

pub(super) fn parse_worktree_changes(bytes: &[u8]) -> Result<WorktreeChanges, HeadError> {
    let mut records = bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty());
    let mut changes = WorktreeChanges::default();
    while let Some(record) = records.next() {
        if record.len() < 3 || record[2] != b' ' {
            return Err(HeadError::InvalidGitOutput("worktree status"));
        }
        let index = record[0];
        let worktree = record[1];
        if index == b'?' && worktree == b'?' {
            changes.untracked += 1;
            continue;
        }
        if index == b'D' || worktree == b'D' {
            changes.deleted += 1;
        } else if index == b'A' || worktree == b'A' {
            changes.added += 1;
        } else {
            changes.modified += 1;
        }
        if matches!(index, b'R' | b'C') || matches!(worktree, b'R' | b'C') {
            records
                .next()
                .ok_or(HeadError::InvalidGitOutput("renamed worktree path"))?;
        }
    }
    Ok(changes)
}

#[cfg(unix)]
#[allow(clippy::unnecessary_wraps)]
pub(super) fn bytes_to_path(value: &[u8]) -> Result<PathBuf, HeadError> {
    Ok(PathBuf::from(OsString::from_vec(value.to_vec())))
}

#[cfg(not(unix))]
pub(super) fn bytes_to_path(value: &[u8]) -> Result<PathBuf, HeadError> {
    let value = std::str::from_utf8(value).map_err(|_| HeadError::InvalidGitOutput("Git path"))?;
    Ok(PathBuf::from(value))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{parse_registered_worktrees, parse_worktree_changes};

    #[test]
    fn porcelain_worktrees_keep_paths_and_symbolic_branches_together() {
        let worktrees = parse_registered_worktrees(
            b"worktree /projects/demo\0HEAD 1111\0branch refs/heads/main\0\0\
              worktree /projects/demo.heads/payment\0HEAD 2222\0branch refs/heads/hydra/payment\0\0",
        )
        .expect("porcelain worktrees should parse");

        assert_eq!(worktrees.len(), 2);
        assert_eq!(worktrees[0].path, Path::new("/projects/demo"));
        assert_eq!(worktrees[0].branch.as_deref(), Some("refs/heads/main"));
        assert_eq!(worktrees[1].path, Path::new("/projects/demo.heads/payment"));
        assert_eq!(
            worktrees[1].branch.as_deref(),
            Some("refs/heads/hydra/payment")
        );
    }

    #[test]
    fn porcelain_worktrees_represent_detached_heads_without_a_branch() {
        let worktrees =
            parse_registered_worktrees(b"worktree /projects/detached\0HEAD 1111\0detached\0\0")
                .expect("detached worktree should parse");

        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, Path::new("/projects/detached"));
        assert_eq!(worktrees[0].branch, None);
    }

    #[test]
    fn porcelain_rename_counts_once_as_modified() {
        let changes = parse_worktree_changes(b"R  new name\0old name\0")
            .expect("a NUL-delimited rename should be valid");

        assert_eq!(changes.modified, 1);
        assert_eq!(changes.added, 0);
        assert_eq!(changes.deleted, 0);
        assert_eq!(changes.untracked, 0);
    }
}

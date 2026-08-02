use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{
    HeadError,
    git::{self, Repository},
    state::{HeadMetadata, StateSnapshot, discover_project_repository},
    validate_head_name,
};

#[derive(Debug)]
pub struct ProjectInspection {
    pub repository_root: PathBuf,
    pub heads_directory: PathBuf,
    pub heads: Vec<HeadSummary>,
}

#[derive(Debug)]
pub struct HeadSummary {
    pub name: String,
    pub status: &'static str,
}

#[derive(Debug)]
pub struct HeadInspection {
    pub name: String,
    pub path: PathBuf,
    pub recorded_head_ref: String,
    pub worktree_head: WorktreeHead,
    pub commit: Option<String>,
    pub base_ref: String,
    pub base_commit: String,
    pub target_ref: String,
    pub materialization_backend: String,
    pub created_at: String,
    pub changes: Option<ChangeCounts>,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    pub worktree_present: bool,
    pub consistency_issues: Vec<String>,
}

#[derive(Debug)]
pub enum WorktreeHead {
    Branch(String),
    Detached,
    Unavailable,
}

#[derive(Debug)]
pub struct ChangeCounts {
    pub modified: usize,
    pub added: usize,
    pub deleted: usize,
    pub untracked: usize,
}

/// Lists the logical names in the validated local Head inventory.
///
/// # Errors
///
/// Returns [`HeadError`] when the repository or Hydra installation cannot be
/// discovered and validated, or when inventory metadata is malformed or
/// unsafe.
pub fn list_heads(source_path: &Path) -> Result<Vec<String>, HeadError> {
    let repository = discover_project_repository(source_path)?;
    let snapshot = StateSnapshot::load(&repository)?;
    let heads_directory = snapshot.heads_directory()?;
    for (name, metadata) in snapshot.heads() {
        validated_head_path(&heads_directory, name, metadata)?;
    }
    Ok(snapshot.heads().keys().cloned().collect())
}

/// Resolves the validated absolute path recorded for a local Head.
///
/// # Errors
///
/// Returns [`HeadError`] when the repository or installation is invalid, the
/// Head is unknown, or its recorded path escapes the owned Heads directory.
pub fn head_path(source_path: &Path, name: &str) -> Result<PathBuf, HeadError> {
    let repository = discover_project_repository(source_path)?;
    let snapshot = StateSnapshot::load(&repository)?;
    let heads_directory = snapshot.heads_directory()?;
    validated_head_path(&heads_directory, name, snapshot.head(name)?)
}

/// Inspects the project and summarizes every locally recorded Head.
///
/// # Errors
///
/// Returns [`HeadError`] when repository discovery, installation validation,
/// inventory parsing, or a required Git read fails.
pub fn inspect_project(source_path: &Path) -> Result<ProjectInspection, HeadError> {
    let repository = discover_project_repository(source_path)?;
    let snapshot = StateSnapshot::load(&repository)?;
    let heads_directory = snapshot.heads_directory()?;
    let registered_worktrees = git::worktree_paths(&repository)?;
    let mut heads = Vec::with_capacity(snapshot.heads().len());
    for (name, metadata) in snapshot.heads() {
        let inspection = inspect_metadata(
            &repository,
            &registered_worktrees,
            &heads_directory,
            name,
            metadata,
        )?;
        let status = if !inspection.consistency_issues.is_empty() {
            "inconsistent"
        } else if inspection
            .changes
            .as_ref()
            .is_some_and(ChangeCounts::is_clean)
        {
            "clean"
        } else {
            "modified"
        };
        heads.push(HeadSummary {
            name: name.clone(),
            status,
        });
    }
    Ok(ProjectInspection {
        repository_root: repository.root,
        heads_directory,
        heads,
    })
}

/// Inspects recorded intent and observed Git/filesystem state for one Head.
///
/// # Errors
///
/// Returns [`HeadError`] when the installation is invalid, the Head is
/// unknown, its path is unsafe, or a required Git read cannot be completed.
pub fn inspect_head(source_path: &Path, name: &str) -> Result<HeadInspection, HeadError> {
    let repository = discover_project_repository(source_path)?;
    let snapshot = StateSnapshot::load(&repository)?;
    let heads_directory = snapshot.heads_directory()?;
    let registered_worktrees = git::worktree_paths(&repository)?;
    inspect_metadata(
        &repository,
        &registered_worktrees,
        &heads_directory,
        name,
        snapshot.head(name)?,
    )
}

fn inspect_metadata(
    repository: &Repository,
    registered_worktrees: &[PathBuf],
    heads_directory: &Path,
    name: &str,
    metadata: &HeadMetadata,
) -> Result<HeadInspection, HeadError> {
    let path = validated_head_path(heads_directory, name, metadata)?;
    let mut consistency_issues = Vec::new();
    let worktree_present = real_directory_exists(&path)?;
    if !worktree_present {
        consistency_issues.push("worktree path is missing".to_owned());
    }

    let registered = registered_worktrees
        .iter()
        .any(|registered_path| registered_path == &path);
    if !registered {
        consistency_issues.push("worktree is not registered with Git".to_owned());
    }

    let head_ref_exists = git::ref_exists(repository, metadata.head_ref())?;
    if !head_ref_exists {
        consistency_issues.push("Head branch is missing".to_owned());
    }
    if !git::ref_exists(repository, metadata.target_ref())? {
        consistency_issues.push("target ref is missing".to_owned());
    }
    let creation_commit = git::commit_for_ref(repository, metadata.base_commit())?;
    let comparison_base = if metadata.base_ref().starts_with("refs/")
        && !git::ref_exists(repository, metadata.base_ref())?
    {
        consistency_issues.push("base ref is missing".to_owned());
        creation_commit.clone()
    } else if metadata.base_ref().starts_with("refs/") {
        git::commit_for_ref(repository, metadata.base_ref())?
    } else {
        creation_commit
    };

    let mut changes = None;
    let mut worktree_head = WorktreeHead::Unavailable;
    let mut commit = None;
    if worktree_present {
        match git::symbolic_head(&path) {
            Ok(Some(reference)) if reference == metadata.head_ref() => {
                worktree_head = WorktreeHead::Branch(reference);
            }
            Ok(Some(reference)) => {
                worktree_head = WorktreeHead::Branch(reference);
                consistency_issues.push("worktree branch does not match metadata".to_owned());
            }
            Ok(None) => {
                worktree_head = WorktreeHead::Detached;
                consistency_issues.push("worktree is detached".to_owned());
            }
            Err(_) => consistency_issues.push("worktree branch is unreadable".to_owned()),
        }
        match git::worktree_commit(&path) {
            Ok(observed_commit) => commit = Some(observed_commit),
            Err(_) => consistency_issues.push("worktree commit is unreadable".to_owned()),
        }
        match git::worktree_changes(&path) {
            Ok(status) => {
                changes = Some(ChangeCounts {
                    modified: status.modified,
                    added: status.added,
                    deleted: status.deleted,
                    untracked: status.untracked,
                });
            }
            Err(_) => consistency_issues.push("worktree status is unreadable".to_owned()),
        }
    }

    let (ahead, behind) = if let Some(observed_commit) = &commit {
        let (ahead, behind) = git::ahead_behind(repository, &comparison_base, observed_commit)?;
        (Some(ahead), Some(behind))
    } else {
        (None, None)
    };

    Ok(HeadInspection {
        name: name.to_owned(),
        path,
        recorded_head_ref: metadata.head_ref().to_owned(),
        worktree_head,
        commit,
        base_ref: metadata.base_ref().to_owned(),
        base_commit: metadata.base_commit().to_owned(),
        target_ref: metadata.target_ref().to_owned(),
        materialization_backend: metadata.materialization_backend().to_owned(),
        created_at: metadata.created_at().to_owned(),
        changes,
        ahead,
        behind,
        worktree_present,
        consistency_issues,
    })
}

pub(super) fn validated_head_path(
    heads_directory: &Path,
    name: &str,
    metadata: &HeadMetadata,
) -> Result<PathBuf, HeadError> {
    validate_head_name(name)?;
    let path = PathBuf::from(metadata.worktree_path());
    if !path.is_absolute() || path != heads_directory.join(name) {
        Err(HeadError::UnsafeHeadPath(path))
    } else {
        Ok(path)
    }
}

fn real_directory_exists(path: &Path) -> Result<bool, HeadError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.is_dir()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(HeadError::FileSystem {
            action: "inspect Head worktree",
            path: path.to_path_buf(),
            source,
        }),
    }
}

impl ChangeCounts {
    fn is_clean(&self) -> bool {
        self.modified == 0 && self.added == 0 && self.deleted == 0 && self.untracked == 0
    }
}

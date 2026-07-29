use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use super::{
    HeadError,
    git::{self, RegisteredWorktree, Repository},
    inspection::validated_head_path,
    state::{HeadMetadata, StateSnapshot, StateTransaction},
};

#[derive(Debug)]
pub struct RepairPlan {
    pub issues: Vec<RepairIssue>,
    pub stale_inventory: Vec<String>,
    pub moved_worktrees: Vec<String>,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum RepairIssue {
    StaleInventory {
        name: String,
        path: PathBuf,
        head_ref: String,
    },
    MovedHeadWorktree {
        name: String,
        recorded_path: PathBuf,
        registered_path: PathBuf,
    },
    UnregisteredHeadDirectory {
        name: String,
        path: PathBuf,
    },
    InvalidHeadDirectory {
        name: String,
        path: PathBuf,
    },
    MissingRegisteredWorktree {
        name: String,
        path: PathBuf,
        head_ref: String,
    },
    MissingHeadBranch {
        name: String,
        head_ref: String,
    },
    WorktreeBranchMismatch {
        name: String,
        path: PathBuf,
        expected_ref: String,
        observed_ref: Option<String>,
    },
    MetadataBranchMismatch {
        name: String,
        recorded_ref: String,
        expected_ref: String,
    },
    AmbiguousHeadWorktrees {
        name: String,
        head_ref: String,
        paths: Vec<PathBuf>,
    },
    UntrackedHydraWorktree {
        name: String,
        path: PathBuf,
        head_ref: String,
    },
}

#[derive(Debug)]
pub struct RepairResult {
    pub removed_stale_inventory: Vec<String>,
    pub restored_worktrees: Vec<String>,
}

/// Compares Hydra inventory with Git worktree and branch state without mutation.
///
/// # Errors
///
/// Returns [`HeadError`] when repository discovery, installation validation,
/// inventory parsing, or Git worktree discovery fails.
pub fn plan_repairs(source_path: &Path) -> Result<RepairPlan, HeadError> {
    let repository = Repository::discover(source_path)?;
    let snapshot = StateSnapshot::load(&repository)?;
    build_plan(
        &repository,
        &snapshot.heads_directory()?,
        snapshot.branch_prefix(),
        snapshot.heads(),
    )
}

/// Removes explicitly approved inventory entries that are still provably stale.
///
/// The private branch is never deleted. Entries that changed after planning are
/// skipped rather than being removed from a newly consistent Head.
///
/// # Errors
///
/// Returns [`HeadError`] when the current project state cannot be validated,
/// locked, inspected, or published atomically.
pub fn apply_repairs(
    source_path: &Path,
    approved_stale_inventory: &[String],
    approved_moved_worktrees: &[String],
) -> Result<RepairResult, HeadError> {
    let repository = Repository::discover(source_path)?;
    let transaction = StateTransaction::open(&repository)?;
    let plan = match build_plan(
        &repository,
        &transaction.heads_directory()?,
        transaction.branch_prefix(),
        transaction.heads(),
    ) {
        Ok(plan) => plan,
        Err(error) => return Err(transaction.abort(error)),
    };
    let approved: BTreeSet<&str> = approved_stale_inventory
        .iter()
        .map(String::as_str)
        .collect();
    let approved_moved: BTreeSet<&str> = approved_moved_worktrees
        .iter()
        .map(String::as_str)
        .collect();
    let approved_current_moved: BTreeSet<&str> = plan
        .moved_worktrees
        .iter()
        .filter_map(|name| {
            approved_moved
                .contains(name.as_str())
                .then_some(name.as_str())
        })
        .collect();
    let mut restored_worktrees = Vec::new();
    for issue in &plan.issues {
        let RepairIssue::MovedHeadWorktree {
            name,
            recorded_path,
            registered_path,
        } = issue
        else {
            continue;
        };
        if approved_current_moved.contains(name.as_str()) {
            if let Err(error) =
                git::move_registered_worktree(&repository, registered_path, recorded_path)
            {
                return Err(transaction.abort(error));
            }
            restored_worktrees.push(name.clone());
        }
    }
    let removed_stale_inventory: Vec<String> = plan
        .stale_inventory
        .into_iter()
        .filter(|name| approved.contains(name.as_str()))
        .collect();

    if removed_stale_inventory.is_empty() {
        transaction.release()?;
    } else {
        transaction.remove_many(&removed_stale_inventory)?;
    }

    Ok(RepairResult {
        removed_stale_inventory,
        restored_worktrees,
    })
}

fn build_plan(
    repository: &Repository,
    heads_directory: &Path,
    branch_prefix: &str,
    heads: &BTreeMap<String, HeadMetadata>,
) -> Result<RepairPlan, HeadError> {
    let worktrees = git::registered_worktrees(repository)?;
    let mut issues = Vec::new();
    let mut stale_inventory = Vec::new();
    let mut moved_worktrees = Vec::new();
    let expected_head_refs: BTreeSet<&str> = heads.values().map(HeadMetadata::head_ref).collect();

    for (name, metadata) in heads {
        let head_plan = plan_recorded_head(
            repository,
            heads_directory,
            branch_prefix,
            name,
            metadata,
            &worktrees,
        )?;
        issues.extend(head_plan.issues);
        if head_plan.stale_inventory {
            stale_inventory.push(name.clone());
        }
        if head_plan.moved_worktree {
            moved_worktrees.push(name.clone());
        }
    }

    let full_branch_prefix = format!("refs/heads/{branch_prefix}");
    for worktree in &worktrees {
        let Some(head_ref) = worktree.branch.as_deref() else {
            continue;
        };
        if expected_head_refs.contains(head_ref) {
            continue;
        }
        let Some(name) = head_ref.strip_prefix(&full_branch_prefix) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        issues.push(RepairIssue::UntrackedHydraWorktree {
            name: name.to_owned(),
            path: worktree.path.clone(),
            head_ref: head_ref.to_owned(),
        });
    }

    Ok(RepairPlan {
        issues,
        stale_inventory,
        moved_worktrees,
    })
}

struct RecordedHeadPlan {
    issues: Vec<RepairIssue>,
    stale_inventory: bool,
    moved_worktree: bool,
}

impl RecordedHeadPlan {
    fn issue(issue: RepairIssue) -> Self {
        Self {
            issues: vec![issue],
            stale_inventory: false,
            moved_worktree: false,
        }
    }
}

fn plan_recorded_head(
    repository: &Repository,
    heads_directory: &Path,
    branch_prefix: &str,
    name: &str,
    metadata: &HeadMetadata,
    worktrees: &[RegisteredWorktree],
) -> Result<RecordedHeadPlan, HeadError> {
    let path = validated_head_path(heads_directory, name, metadata)?;
    let expected_ref = format!("refs/heads/{branch_prefix}{name}");
    if metadata.head_ref() != expected_ref {
        return Ok(RecordedHeadPlan::issue(
            RepairIssue::MetadataBranchMismatch {
                name: name.to_owned(),
                recorded_ref: metadata.head_ref().to_owned(),
                expected_ref,
            },
        ));
    }

    let branch_exists = git::ref_exists(repository, metadata.head_ref())?;
    if let Some(worktree) = worktrees.iter().find(|worktree| worktree.path == path) {
        return plan_exact_worktree(name, metadata, path, worktree, branch_exists);
    }

    let matching_worktrees: Vec<&RegisteredWorktree> = worktrees
        .iter()
        .filter(|worktree| worktree.branch.as_deref() == Some(metadata.head_ref()))
        .collect();
    match matching_worktrees.as_slice() {
        [worktree] => {
            let moved_worktree = path_is_missing(&path, "inspect recorded Head during repair")?;
            Ok(RecordedHeadPlan {
                issues: vec![RepairIssue::MovedHeadWorktree {
                    name: name.to_owned(),
                    recorded_path: path,
                    registered_path: worktree.path.clone(),
                }],
                stale_inventory: false,
                moved_worktree,
            })
        }
        [] => plan_unregistered_head(name, metadata, path, branch_exists),
        _ => Ok(RecordedHeadPlan::issue(
            RepairIssue::AmbiguousHeadWorktrees {
                name: name.to_owned(),
                head_ref: metadata.head_ref().to_owned(),
                paths: matching_worktrees
                    .into_iter()
                    .map(|worktree| worktree.path.clone())
                    .collect(),
            },
        )),
    }
}

fn plan_exact_worktree(
    name: &str,
    metadata: &HeadMetadata,
    path: PathBuf,
    worktree: &RegisteredWorktree,
    branch_exists: bool,
) -> Result<RecordedHeadPlan, HeadError> {
    if path_is_missing(&path, "inspect registered Head during repair")? {
        return Ok(RecordedHeadPlan::issue(
            RepairIssue::MissingRegisteredWorktree {
                name: name.to_owned(),
                path,
                head_ref: metadata.head_ref().to_owned(),
            },
        ));
    }
    if !fs::symlink_metadata(&path)
        .map_err(|source| HeadError::FileSystem {
            action: "inspect registered Head during repair",
            path: path.clone(),
            source,
        })?
        .is_dir()
    {
        return Ok(RecordedHeadPlan::issue(RepairIssue::InvalidHeadDirectory {
            name: name.to_owned(),
            path,
        }));
    }

    let mut issues = Vec::new();
    if worktree.branch.as_deref() != Some(metadata.head_ref()) {
        issues.push(RepairIssue::WorktreeBranchMismatch {
            name: name.to_owned(),
            path,
            expected_ref: metadata.head_ref().to_owned(),
            observed_ref: worktree.branch.clone(),
        });
    }
    if !branch_exists {
        issues.push(RepairIssue::MissingHeadBranch {
            name: name.to_owned(),
            head_ref: metadata.head_ref().to_owned(),
        });
    }
    Ok(RecordedHeadPlan {
        issues,
        stale_inventory: false,
        moved_worktree: false,
    })
}

fn plan_unregistered_head(
    name: &str,
    metadata: &HeadMetadata,
    path: PathBuf,
    branch_exists: bool,
) -> Result<RecordedHeadPlan, HeadError> {
    match fs::symlink_metadata(&path) {
        Ok(path_metadata) if path_metadata.is_dir() => Ok(RecordedHeadPlan::issue(
            RepairIssue::UnregisteredHeadDirectory {
                name: name.to_owned(),
                path,
            },
        )),
        Ok(_) => Ok(RecordedHeadPlan::issue(RepairIssue::InvalidHeadDirectory {
            name: name.to_owned(),
            path,
        })),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && branch_exists => {
            Ok(RecordedHeadPlan {
                issues: vec![RepairIssue::StaleInventory {
                    name: name.to_owned(),
                    path,
                    head_ref: metadata.head_ref().to_owned(),
                }],
                stale_inventory: true,
                moved_worktree: false,
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(RecordedHeadPlan::issue(RepairIssue::MissingHeadBranch {
                name: name.to_owned(),
                head_ref: metadata.head_ref().to_owned(),
            }))
        }
        Err(source) => Err(HeadError::FileSystem {
            action: "inspect recorded Head during repair",
            path,
            source,
        }),
    }
}

fn path_is_missing(path: &Path, action: &'static str) -> Result<bool, HeadError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(source) => Err(HeadError::FileSystem {
            action,
            path: path.to_path_buf(),
            source,
        }),
    }
}

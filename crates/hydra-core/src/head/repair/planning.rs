use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use super::super::{
    HeadError,
    git::{self, RegisteredWorktree, Repository},
    inspection::validated_head_path,
    persistence::{self, StateLockInspection},
    recovery,
    state::{HeadMetadata, MissingStateSnapshot},
};
use super::{RepairIssue, RepairPlan};

pub(super) fn build_plan(
    repository: &Repository,
    heads_directory: &Path,
    branch_prefix: &str,
    heads: &BTreeMap<String, HeadMetadata>,
) -> Result<RepairPlan, HeadError> {
    build_present_repair_state(repository, heads_directory, branch_prefix, heads)
        .map(|(plan, _)| plan)
}

pub(super) fn build_present_repair_state(
    repository: &Repository,
    heads_directory: &Path,
    branch_prefix: &str,
    heads: &BTreeMap<String, HeadMetadata>,
) -> Result<(RepairPlan, BTreeMap<String, HeadMetadata>), HeadError> {
    let worktrees = git::registered_worktrees(repository)?;
    let mut issues = Vec::new();
    let mut stale_inventory = Vec::new();
    let mut moved_worktrees = Vec::new();
    let mut recoverable_untracked_heads = Vec::new();
    let mut recovered_untracked_heads = BTreeMap::new();
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
        let (issue, recovered) = plan_untracked_hydra_worktree(
            repository,
            heads_directory,
            heads,
            name,
            head_ref,
            worktree,
        )?;
        issues.push(issue);
        if let Some(metadata) = recovered {
            recoverable_untracked_heads.push(name.to_owned());
            recovered_untracked_heads.insert(name.to_owned(), metadata);
        }
    }

    let (pending_issues, recoverable_pending_creations) = plan_pending_creations(
        repository,
        heads_directory,
        branch_prefix,
        heads,
        &worktrees,
    )?;
    issues.extend(pending_issues);

    Ok((
        RepairPlan {
            issues,
            stale_inventory,
            moved_worktrees,
            missing_inventory: None,
            recoverable_inventory: Vec::new(),
            abandoned_state_lock: None,
            recoverable_untracked_heads,
            recoverable_pending_creations,
        },
        recovered_untracked_heads,
    ))
}

fn plan_untracked_hydra_worktree(
    repository: &Repository,
    heads_directory: &Path,
    heads: &BTreeMap<String, HeadMetadata>,
    name: &str,
    head_ref: &str,
    worktree: &RegisteredWorktree,
) -> Result<(RepairIssue, Option<HeadMetadata>), HeadError> {
    let report_only = || {
        (
            RepairIssue::UntrackedHydraWorktree {
                name: name.to_owned(),
                path: worktree.path.clone(),
                head_ref: head_ref.to_owned(),
            },
            None,
        )
    };
    if heads.contains_key(name) || !git::ref_exists(repository, head_ref)? {
        return Ok(report_only());
    }
    match fs::symlink_metadata(&worktree.path) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Ok(report_only()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(report_only()),
        Err(source) => {
            return Err(HeadError::FileSystem {
                action: "inspect untracked Hydra worktree",
                path: worktree.path.clone(),
                source,
            });
        }
    }
    let Some(recovered) = read_head_recovery(repository, heads_directory, name, &worktree.path)?
    else {
        return Ok(report_only());
    };
    let recovered_path = validated_head_path(heads_directory, name, &recovered.metadata)?;
    if recovered.name != name
        || recovered.metadata.head_ref() != head_ref
        || recovered_path != worktree.path
    {
        return Ok(report_only());
    }
    Ok((
        RepairIssue::RecoverableUntrackedHead {
            name: name.to_owned(),
            path: recovered_path,
            head_ref: head_ref.to_owned(),
        },
        Some(recovered.metadata),
    ))
}

pub(super) fn build_missing_inventory_plan(
    repository: &Repository,
    snapshot: &MissingStateSnapshot,
) -> Result<RepairPlan, HeadError> {
    build_missing_inventory_state(
        repository,
        snapshot.state_path(),
        &snapshot.heads_directory()?,
        snapshot.branch_prefix(),
    )
    .map(|(plan, _)| plan)
}

pub(super) fn build_missing_inventory_state(
    repository: &Repository,
    state_path: &Path,
    heads_directory: &Path,
    branch_prefix: &str,
) -> Result<(RepairPlan, BTreeMap<String, HeadMetadata>), HeadError> {
    let state_path = state_path.to_path_buf();
    let full_branch_prefix = format!("refs/heads/{branch_prefix}");
    let mut issues = vec![RepairIssue::MissingInventory {
        path: state_path.clone(),
    }];
    let mut recoverable_inventory = Vec::new();
    let mut recovered_inventory = BTreeMap::new();
    let mut all_recoverable = true;

    for worktree in git::registered_worktrees(repository)? {
        let Some(head_ref) = worktree.branch.as_deref() else {
            continue;
        };
        let Some(name) = head_ref.strip_prefix(&full_branch_prefix) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let Some(recovered) =
            read_head_recovery(repository, heads_directory, name, &worktree.path)?
        else {
            all_recoverable = false;
            issues.push(RepairIssue::UntrackedHydraWorktree {
                name: name.to_owned(),
                path: worktree.path,
                head_ref: head_ref.to_owned(),
            });
            continue;
        };
        let recovered_path = validated_head_path(heads_directory, name, &recovered.metadata)?;
        if recovered.name != name
            || recovered.metadata.head_ref() != head_ref
            || recovered_path != worktree.path
        {
            all_recoverable = false;
            issues.push(RepairIssue::UntrackedHydraWorktree {
                name: name.to_owned(),
                path: worktree.path,
                head_ref: head_ref.to_owned(),
            });
            continue;
        }
        issues.push(RepairIssue::RecoverableHead {
            name: name.to_owned(),
            path: recovered_path,
            head_ref: head_ref.to_owned(),
        });
        recoverable_inventory.push(name.to_owned());
        recovered_inventory.insert(name.to_owned(), recovered.metadata);
    }

    if !all_recoverable || recoverable_inventory.is_empty() {
        recoverable_inventory.clear();
        recovered_inventory.clear();
    }

    Ok((
        RepairPlan {
            issues,
            stale_inventory: Vec::new(),
            moved_worktrees: Vec::new(),
            missing_inventory: Some(state_path),
            recoverable_inventory,
            abandoned_state_lock: None,
            recoverable_untracked_heads: Vec::new(),
            recoverable_pending_creations: Vec::new(),
        },
        recovered_inventory,
    ))
}

fn read_head_recovery(
    repository: &Repository,
    heads_directory: &Path,
    name: &str,
    worktree: &Path,
) -> Result<Option<recovery::RecoveredHead>, HeadError> {
    let private = recovery::read_manifest(repository, worktree)?;
    let central = recovery::read_central_recovery(heads_directory, name)?;
    match (private, central) {
        (Some(private), Some(central))
            if private.name == central.name && private.metadata == central.metadata =>
        {
            Ok(Some(private))
        }
        (Some(recovered), None) | (None, Some(recovered)) => Ok(Some(recovered)),
        (None, None) | (Some(_), Some(_)) => Ok(None),
    }
}

fn plan_pending_creations(
    repository: &Repository,
    heads_directory: &Path,
    branch_prefix: &str,
    heads: &BTreeMap<String, HeadMetadata>,
    worktrees: &[RegisteredWorktree],
) -> Result<(Vec<RepairIssue>, Vec<String>), HeadError> {
    let mut issues = Vec::new();
    let mut recoverable = Vec::new();
    for pending in recovery::read_pending_creations(heads_directory)? {
        super::super::validate_head_name(pending.name())?;
        let expected_path = heads_directory.join(pending.name());
        let expected_ref = format!("refs/heads/{branch_prefix}{}", pending.name());
        if pending.intent().worktree_path() != expected_path
            || pending.intent().head_ref() != expected_ref
        {
            return Err(HeadError::UnsafeHeadPath(
                pending.intent().worktree_path().to_path_buf(),
            ));
        }
        let already_recorded = heads.contains_key(pending.name());
        let has_worktree = worktrees.iter().any(|worktree| {
            worktree.path == expected_path || worktree.branch.as_deref() == Some(&expected_ref)
        });
        let path_exists = expected_path
            .try_exists()
            .map_err(|source| HeadError::FileSystem {
                action: "inspect pending Head path",
                path: expected_path.clone(),
                source,
            })?;
        let branch_exists = git::ref_exists(repository, &expected_ref)?;
        let branch_is_unchanged = !branch_exists
            || git::commit_for_ref(repository, &expected_ref)? == pending.intent().base_commit();
        issues.push(RepairIssue::IncompleteHeadCreation {
            name: pending.name().to_owned(),
            path: expected_path,
            head_ref: expected_ref,
        });
        if already_recorded || (!has_worktree && !path_exists && branch_is_unchanged) {
            recoverable.push(pending.name().to_owned());
        }
    }
    Ok((issues, recoverable))
}

pub(super) fn attach_state_lock_issue(
    plan: &mut RepairPlan,
    state_path: &Path,
) -> Result<(), HeadError> {
    match persistence::inspect_state_lock(state_path)? {
        StateLockInspection::Absent => {}
        StateLockInspection::Active(path) => {
            plan.issues.push(RepairIssue::ActiveStateLock { path });
        }
        StateLockInspection::Abandoned(path) => {
            plan.issues
                .push(RepairIssue::AbandonedStateLock { path: path.clone() });
            plan.abandoned_state_lock = Some(path);
        }
    }
    Ok(())
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

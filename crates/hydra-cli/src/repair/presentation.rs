use std::io::{self, BufRead, Write};

use crate::output::safe_path_label;

pub(super) fn show_issue(issue: &hydra_core::RepairIssue) {
    match issue {
        hydra_core::RepairIssue::MissingInventory { path } => {
            println!("Missing Hydra inventory: {}", safe_path_label(path));
        }
        hydra_core::RepairIssue::RecoverableHead {
            name,
            path: _,
            head_ref: _,
        } => println!("Recoverable Head: {name}"),
        hydra_core::RepairIssue::AbandonedStateLock { path } => {
            println!("Abandoned Hydra state lock: {}", safe_path_label(path));
        }
        hydra_core::RepairIssue::ActiveStateLock { path } => {
            println!("Active Hydra state lock: {}", safe_path_label(path));
        }
        hydra_core::RepairIssue::RecoverableUntrackedHead {
            name,
            path: _,
            head_ref: _,
        } => println!("Recoverable untracked Head: {name}"),
        hydra_core::RepairIssue::IncompleteHeadCreation {
            name,
            path: _,
            head_ref: _,
        } => println!("Incomplete Head creation: {name}"),
        hydra_core::RepairIssue::StaleInventory {
            name,
            path,
            head_ref,
        } => println!(
            "Stale inventory: {name} (missing {}, preserving {head_ref})",
            safe_path_label(path)
        ),
        hydra_core::RepairIssue::MovedHeadWorktree {
            name,
            recorded_path: _,
            registered_path,
        } => println!(
            "Moved Head worktree: {name} is registered at {}",
            safe_path_label(registered_path)
        ),
        hydra_core::RepairIssue::UnregisteredHeadDirectory { name, path } => println!(
            "Unregistered Head directory: {name} at {}; manual recovery required",
            safe_path_label(path)
        ),
        hydra_core::RepairIssue::InvalidHeadDirectory { name, path } => println!(
            "Invalid Head directory: {name} at {}; manual recovery required",
            safe_path_label(path)
        ),
        hydra_core::RepairIssue::MissingRegisteredWorktree {
            name,
            path,
            head_ref: _,
        } => println!(
            "Missing registered worktree: {name} at {}; manual recovery required",
            safe_path_label(path)
        ),
        hydra_core::RepairIssue::MissingHeadBranch { name, head_ref } => {
            println!("Missing Head branch: {name} expects {head_ref}; manual recovery required");
        }
        hydra_core::RepairIssue::WorktreeBranchMismatch {
            name,
            path,
            expected_ref,
            observed_ref,
        } => println!(
            "Worktree branch mismatch: {name} at {} expects {expected_ref}, observed {}; manual recovery required",
            safe_path_label(path),
            observed_ref.as_deref().unwrap_or("detached HEAD")
        ),
        hydra_core::RepairIssue::MetadataBranchMismatch {
            name,
            recorded_ref,
            expected_ref,
        } => println!(
            "Metadata branch mismatch: {name} records {recorded_ref}, expected {expected_ref}; manual recovery required"
        ),
        hydra_core::RepairIssue::AmbiguousHeadWorktrees {
            name,
            head_ref,
            paths,
        } => println!(
            "Ambiguous Head worktrees: {name} branch {head_ref} appears at {} location(s); manual recovery required",
            paths.len()
        ),
        hydra_core::RepairIssue::UntrackedHydraWorktree {
            name,
            path,
            head_ref: _,
        } => println!(
            "Untracked Hydra worktree: {name} at {}; manual recovery required",
            safe_path_label(path)
        ),
        _ => println!("Unknown repair issue; manual recovery required"),
    }
}

pub(super) fn request_abandoned_lock_confirmation(
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> io::Result<bool> {
    write!(output, "Remove the abandoned Hydra state lock? [y/N] ")?;
    output.flush()?;
    read_confirmation(input)
}

pub(super) fn request_untracked_head_recovery_confirmation(
    input: &mut impl BufRead,
    output: &mut impl Write,
    count: usize,
) -> io::Result<bool> {
    if count == 1 {
        write!(output, "Add 1 recovered Head to the inventory? [y/N] ")?;
    } else {
        write!(
            output,
            "Add {count} recovered Heads to the inventory? [y/N] "
        )?;
    }
    output.flush()?;
    read_confirmation(input)
}

pub(super) fn request_pending_creation_confirmation(
    input: &mut impl BufRead,
    output: &mut impl Write,
    count: usize,
) -> io::Result<bool> {
    if count == 1 {
        write!(output, "Clean up 1 incomplete Head creation? [y/N] ")?;
    } else {
        write!(output, "Clean up {count} incomplete Head creations? [y/N] ")?;
    }
    output.flush()?;
    read_confirmation(input)
}

pub(super) fn request_inventory_recovery_confirmation(
    input: &mut impl BufRead,
    output: &mut impl Write,
    count: usize,
) -> io::Result<bool> {
    if count == 1 {
        write!(
            output,
            "Rebuild the missing inventory with 1 recovered Head? [y/N] "
        )?;
    } else {
        write!(
            output,
            "Rebuild the missing inventory with {count} recovered Heads? [y/N] "
        )?;
    }
    output.flush()?;
    read_confirmation(input)
}

pub(super) fn request_stale_inventory_confirmation(
    input: &mut impl BufRead,
    output: &mut impl Write,
    count: usize,
) -> io::Result<bool> {
    if count == 1 {
        write!(
            output,
            "Remove 1 stale inventory entry while preserving its branch? [y/N] "
        )?;
    } else {
        write!(
            output,
            "Remove {count} stale inventory entries while preserving their branches? [y/N] "
        )?;
    }
    output.flush()?;
    read_confirmation(input)
}

pub(super) fn request_moved_worktree_confirmation(
    input: &mut impl BufRead,
    output: &mut impl Write,
    count: usize,
) -> io::Result<bool> {
    if count == 1 {
        write!(
            output,
            "Move 1 relocated Head worktree back to its managed path? [y/N] "
        )?;
    } else {
        write!(
            output,
            "Move {count} relocated Head worktrees back to their managed paths? [y/N] "
        )?;
    }
    output.flush()?;
    read_confirmation(input)
}

fn read_confirmation(input: &mut impl BufRead) -> io::Result<bool> {
    let mut response = String::new();
    input.read_line(&mut response)?;
    Ok(matches!(
        response.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

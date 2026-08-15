use std::{io, path::Path, process::ExitCode};

mod presentation;

use presentation::{
    request_abandoned_lock_confirmation, request_inventory_recovery_confirmation,
    request_moved_worktree_confirmation, request_pending_creation_confirmation,
    request_stale_inventory_confirmation, request_untracked_head_recovery_confirmation, show_issue,
};

pub(super) fn run() -> ExitCode {
    let plan = match hydra_core::plan_repairs(Path::new(".")) {
        Ok(plan) => plan,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    if plan.issues.is_empty() {
        println!("Hydra state is consistent.");
        return ExitCode::SUCCESS;
    }

    for issue in &plan.issues {
        show_issue(issue);
    }
    if plan.abandoned_state_lock.is_some() {
        return repair_abandoned_state_lock();
    }
    if plan.missing_inventory.is_some() {
        return repair_missing_inventory(&plan);
    }
    if !plan.recoverable_pending_creations.is_empty() {
        return repair_pending_creations(&plan);
    }
    if !plan.recoverable_untracked_heads.is_empty() {
        return repair_untracked_heads(&plan);
    }
    if plan.stale_inventory.is_empty() && plan.moved_worktrees.is_empty() {
        println!("No automatic repairs available; manual recovery required.");
        return ExitCode::SUCCESS;
    }

    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let restore_allowed = if plan.moved_worktrees.is_empty() {
        false
    } else {
        match request_moved_worktree_confirmation(
            &mut input,
            &mut output,
            plan.moved_worktrees.len(),
        ) {
            Ok(confirmed) => confirmed,
            Err(error) => {
                eprintln!("error: failed to read repair confirmation: {error}");
                return ExitCode::FAILURE;
            }
        }
    };
    let cleanup_allowed = if plan.stale_inventory.is_empty() {
        false
    } else {
        match request_stale_inventory_confirmation(
            &mut input,
            &mut output,
            plan.stale_inventory.len(),
        ) {
            Ok(confirmed) => confirmed,
            Err(error) => {
                eprintln!("error: failed to read repair confirmation: {error}");
                return ExitCode::FAILURE;
            }
        }
    };
    if !restore_allowed && !cleanup_allowed {
        println!("No repairs applied.");
        return ExitCode::SUCCESS;
    }

    let restore_names = if restore_allowed {
        plan.moved_worktrees.as_slice()
    } else {
        &[]
    };
    let cleanup_names = if cleanup_allowed {
        plan.stale_inventory.as_slice()
    } else {
        &[]
    };
    match hydra_core::apply_repairs(Path::new("."), cleanup_names, restore_names) {
        Ok(result) => {
            let restored = result.restored_worktrees.len();
            if restored == 1 {
                println!("Restored 1 Head worktree to its managed path.");
            } else if restored > 1 {
                println!("Restored {restored} Head worktrees to their managed paths.");
            }
            let removed = result.removed_stale_inventory.len();
            if removed == 1 {
                println!("Removed 1 stale inventory entry.");
            } else if removed > 1 {
                println!("Removed {removed} stale inventory entries.");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn repair_pending_creations(plan: &hydra_core::RepairPlan) -> ExitCode {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    match request_pending_creation_confirmation(
        &mut input,
        &mut output,
        plan.recoverable_pending_creations.len(),
    ) {
        Ok(true) => match hydra_core::apply_pending_creation_recovery(
            Path::new("."),
            &plan.recoverable_pending_creations,
        ) {
            Ok(result) if result.cleaned_creations.len() == 1 => {
                println!("Cleaned up 1 incomplete Head creation.");
                ExitCode::SUCCESS
            }
            Ok(result) if !result.cleaned_creations.is_empty() => {
                println!(
                    "Cleaned up {} incomplete Head creations.",
                    result.cleaned_creations.len()
                );
                ExitCode::SUCCESS
            }
            Ok(_) => {
                println!("No repairs applied; pending creation state changed during confirmation.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(false) => {
            println!("No repairs applied.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: failed to read repair confirmation: {error}");
            ExitCode::FAILURE
        }
    }
}

fn repair_untracked_heads(plan: &hydra_core::RepairPlan) -> ExitCode {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    match request_untracked_head_recovery_confirmation(
        &mut input,
        &mut output,
        plan.recoverable_untracked_heads.len(),
    ) {
        Ok(true) => match hydra_core::apply_untracked_head_recovery(
            Path::new("."),
            &plan.recoverable_untracked_heads,
        ) {
            Ok(result) if result.recovered_heads.len() == 1 => {
                println!("Added 1 recovered Head to the inventory.");
                ExitCode::SUCCESS
            }
            Ok(result) if !result.recovered_heads.is_empty() => {
                println!(
                    "Added {} recovered Heads to the inventory.",
                    result.recovered_heads.len()
                );
                ExitCode::SUCCESS
            }
            Ok(_) => {
                println!("No repairs applied; Head recovery changed during confirmation.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(false) => {
            println!("No repairs applied.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: failed to read repair confirmation: {error}");
            ExitCode::FAILURE
        }
    }
}

fn repair_abandoned_state_lock() -> ExitCode {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    match request_abandoned_lock_confirmation(&mut input, &mut output) {
        Ok(true) => match hydra_core::apply_abandoned_state_lock_recovery(Path::new(".")) {
            Ok(true) => {
                println!("Removed the abandoned Hydra state lock.");
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!("No repairs applied; state-lock recovery changed during confirmation.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(false) => {
            println!("No repairs applied.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: failed to read repair confirmation: {error}");
            ExitCode::FAILURE
        }
    }
}

fn repair_missing_inventory(plan: &hydra_core::RepairPlan) -> ExitCode {
    if plan.recoverable_inventory.is_empty() {
        println!("No automatic repairs available; manual recovery required.");
        return ExitCode::SUCCESS;
    }
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let confirmed = match request_inventory_recovery_confirmation(
        &mut input,
        &mut output,
        plan.recoverable_inventory.len(),
    ) {
        Ok(confirmed) => confirmed,
        Err(error) => {
            eprintln!("error: failed to read repair confirmation: {error}");
            return ExitCode::FAILURE;
        }
    };
    if !confirmed {
        println!("No repairs applied.");
        return ExitCode::SUCCESS;
    }
    match hydra_core::apply_inventory_recovery(Path::new("."), &plan.recoverable_inventory) {
        Ok(result) if result.recovered_heads.len() == 1 => {
            println!("Rebuilt the missing inventory with 1 recovered Head.");
            ExitCode::SUCCESS
        }
        Ok(result) if !result.recovered_heads.is_empty() => {
            println!(
                "Rebuilt the missing inventory with {} recovered Heads.",
                result.recovered_heads.len()
            );
            ExitCode::SUCCESS
        }
        Ok(_) => {
            println!("No repairs applied; recovery state changed during confirmation.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

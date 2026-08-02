use std::{
    ffi::OsStr,
    io::{self, BufRead, IsTerminal, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{CommandFactory as _, Parser, Subcommand, ValueEnum};
use clap_complete::{
    CompleteEnv,
    engine::{ArgValueCompleter, CompletionCandidate},
    env::{Bash, EnvCompleter, Fish, Shells, Zsh},
};

mod inspection;
mod output;

#[derive(Parser)]
#[command(
    name = "hydra",
    version,
    about = "Git-native workspace manager for isolated development Heads",
    long_about = "Git-native workspace manager for isolated development Heads.\n\nHydra creates independent working directories while preserving familiar Git refs, branches, and repository workflows.",
    after_help = "Command syntax:\n  hydra init [PATH]\n  hydra status\n  hydra repair\n  hydra doctor storage\n  hydra completions <SHELL>\n  hydra head create <NAME> [--from <REF>] [--target <BRANCH>]\n  hydra head list\n  hydra head status <NAME>\n  hydra head path <NAME>\n  hydra head open <NAME>\n  hydra head close <NAME>\n  hydra head remove <NAME> [--force]\n\nRun 'hydra <command> --help' for details."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize Hydra in a Git repository
    #[command(after_help = "Examples:\n  hydra init\n  hydra init path/to/repository")]
    Init {
        /// Git repository to initialize
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Show the project and local Heads
    Status,
    /// Reconcile Hydra inventory with Git worktrees
    #[command(
        long_about = "Reconcile Hydra inventory with Git worktrees.\n\nHydra reports ambiguous inconsistencies without mutation and asks for confirmation before applying deterministic repairs.",
        after_help = "Examples:\n  hydra repair"
    )]
    Repair,
    /// Diagnose project capabilities
    #[command(
        after_help = "Command syntax:\n  hydra doctor storage\n\nRun 'hydra doctor <command> --help' for details."
    )]
    Doctor {
        #[command(subcommand)]
        command: DoctorCommand,
    },
    /// Print shell registration for dynamic completions
    #[command(
        long_about = "Print shell registration for Hydra's static and dynamic completions.\n\nThe generated registration completes commands and reads the local Head inventory when an existing Head name is expected.",
        after_help = "Examples:\n  source <(hydra completions bash)\n  source <(hydra completions zsh)\n  hydra completions fish | source"
    )]
    Completions {
        /// Shell that will load the completion registration
        #[arg(value_enum)]
        shell: CompletionShell,
    },
    /// Create and manage Heads
    #[command(
        after_help = "Command syntax:\n  hydra head create <NAME> [--from <REF>] [--target <BRANCH>]\n  hydra head list\n  hydra head status <NAME>\n  hydra head path <NAME>\n  hydra head open <NAME>\n  hydra head close <NAME>\n  hydra head remove <NAME> [--force]\n\nRun 'hydra head <command> --help' for details."
    )]
    Head {
        #[command(subcommand)]
        command: HeadCommand,
    },
    #[command(name = "__complete", hide = true)]
    Complete {
        #[command(subcommand)]
        command: CompletionCommand,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

#[derive(Subcommand)]
enum CompletionCommand {
    Heads,
}

#[derive(Subcommand)]
enum DoctorCommand {
    /// Run a real storage probe on the Heads volume
    #[command(
        long_about = "Run a real storage probe on the Heads volume.\n\nHydra verifies the native copy-on-write primitive and the isolated full-copy fallback with temporary files.",
        after_help = "Examples:\n  hydra doctor storage"
    )]
    Storage,
}

#[derive(Subcommand)]
enum HeadCommand {
    /// Create a new isolated Head
    #[command(
        long_about = "Create a new isolated Head.\n\nThe new Head starts at <REF> and uses <BRANCH> as its local integration branch. When invoked from an existing Hydra Head, configuration, HEAD defaults, and overlays come from the canonical parent project exactly as if the command ran there.",
        after_help = "Examples:\n  hydra head create payment\n  hydra head create payment --from beta\n  hydra head create payment --from beta --target main"
    )]
    Create {
        /// Name for the new Head
        name: String,
        /// Start at this ref or commit (default: canonical parent project HEAD)
        #[arg(long, value_name = "REF")]
        from: Option<String>,
        /// Set the local branch used for integration
        #[arg(long, value_name = "BRANCH")]
        target: Option<String>,
    },
    /// List local Heads
    List,
    /// Show the state of a local Head
    Status {
        /// Name of an existing Head
        #[arg(add = ArgValueCompleter::new(complete_head_names))]
        name: String,
    },
    /// Print the absolute path of a local Head
    Path {
        /// Name of an existing Head
        #[arg(add = ArgValueCompleter::new(complete_head_names))]
        name: String,
    },
    /// Open a local Head with the configured command
    #[command(
        long_about = "Open a local Head with the configured command.\n\nThe Head path and Git branch are validated before the configured adapter is started.",
        after_help = "Examples:\n  hydra head open payment"
    )]
    Open {
        /// Name of an existing Head
        #[arg(add = ArgValueCompleter::new(complete_head_names))]
        name: String,
    },
    /// Remove a local Head safely
    #[command(
        long_about = "Remove a local Head safely.\n\nWithout --force, the Head must be clean and its commits must already be integrated into its target branch.",
        after_help = "Examples:\n  hydra head remove payment\n  hydra head remove payment --force"
    )]
    Remove {
        /// Name of an existing Head
        #[arg(add = ArgValueCompleter::new(complete_head_names))]
        name: String,
        /// Discard uncommitted worktree changes
        #[arg(long)]
        force: bool,
    },
    /// Integrate and remove a completed Head
    #[command(
        long_about = "Integrate or run the configured close adapter for a completed Head.\n\nThe Head must be clean. By default, Hydra integrates through the target when it is checked out in a clean worktree; when the target is not checked out, integration remains checkout-free. A dirty target worktree or an active Git operation blocks close without mutation. Native close reports its integration strategy and result, then performs protected removal. When .hydra.json defines commands.close, Hydra runs that adapter instead; removeOnSuccess controls whether protected removal follows a successful command.",
        after_help = "Examples:\n  hydra head close payment"
    )]
    Close {
        /// Name of an existing Head
        #[arg(add = ArgValueCompleter::new(complete_head_names))]
        name: String,
    },
}

fn main() -> ExitCode {
    let supported_shells: [&dyn EnvCompleter; 3] = [&Bash, &Zsh, &Fish];
    CompleteEnv::with_factory(Cli::command)
        .shells(Shells(&supported_shells))
        .complete();

    let cli = Cli::parse();

    match cli.command {
        Command::Init { path } => match hydra_core::initialize(&path) {
            Ok(project) => {
                println!("Initialized Hydra in {}", project.repository_root.display());
                match project.storage_backend {
                    hydra_core::StorageBackend::CopyOnWrite => {
                        println!("Storage backend: copy-on-write");
                    }
                    hydra_core::StorageBackend::FullCopy => {
                        println!("Storage backend: full copy");
                    }
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        Command::Status => inspection::show_project_status(),
        Command::Repair => repair(),
        Command::Doctor {
            command: DoctorCommand::Storage,
        } => doctor_storage(),
        Command::Completions { shell } => print_completions(shell),
        Command::Head {
            command: HeadCommand::Create { name, from, target },
        } => create_head(&name, from.as_deref(), target.as_deref()),
        Command::Head {
            command: HeadCommand::List,
        } => inspection::list_heads(),
        Command::Head {
            command: HeadCommand::Status { name },
        } => inspection::show_head_status(&name),
        Command::Head {
            command: HeadCommand::Path { name },
        } => inspection::show_head_path(&name),
        Command::Head {
            command: HeadCommand::Open { name },
        } => open_head(&name),
        Command::Head {
            command: HeadCommand::Remove { name, force },
        } => remove_head(&name, force),
        Command::Head {
            command: HeadCommand::Close { name },
        } => close_head(&name),
        Command::Complete {
            command: CompletionCommand::Heads,
        } => print_head_candidates(),
    }
}

fn print_completions(shell: CompletionShell) -> ExitCode {
    match shell {
        CompletionShell::Bash => println!("source <(COMPLETE=bash hydra)"),
        CompletionShell::Zsh => println!("source <(COMPLETE=zsh hydra)"),
        CompletionShell::Fish => println!("COMPLETE=fish hydra | source"),
    }
    ExitCode::SUCCESS
}

fn head_names() -> Vec<String> {
    let mut names = hydra_core::list_heads(Path::new(".")).unwrap_or_default();
    names.sort_unstable();
    names.dedup();
    names
}

fn complete_head_names(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(prefix) = current.to_str() else {
        return Vec::new();
    };
    head_names()
        .into_iter()
        .filter(|name| name.starts_with(prefix))
        .map(CompletionCandidate::new)
        .collect()
}

fn print_head_candidates() -> ExitCode {
    for name in head_names() {
        println!("{name}");
    }
    ExitCode::SUCCESS
}

fn doctor_storage() -> ExitCode {
    match hydra_core::diagnose_storage(Path::new(".")) {
        Ok(diagnostics) => {
            match diagnostics.storage_backend {
                hydra_core::StorageBackend::CopyOnWrite => {
                    println!("Storage backend: copy-on-write");
                }
                hydra_core::StorageBackend::FullCopy => {
                    println!("Storage backend: full copy");
                }
            }
            let primitive = match diagnostics.native_primitive {
                hydra_core::NativeStoragePrimitive::ApfsClone => "APFS clone",
                hydra_core::NativeStoragePrimitive::LinuxReflink => "Linux reflink",
                hydra_core::NativeStoragePrimitive::NativeClone => "native clone",
                hydra_core::NativeStoragePrimitive::Unavailable => "unavailable",
            };
            println!("Native primitive: {primitive}");
            if diagnostics.full_copy_fallback_verified {
                println!("Fallback: full copy (verified)");
            }
            let hard_links = if diagnostics.mutable_hard_links_enabled {
                "enabled"
            } else {
                "disabled"
            };
            println!("Mutable hard links: {hard_links}");
            let isolation = if diagnostics.isolation_supported {
                "supported"
            } else {
                "unsupported"
            };
            println!("Isolation: {isolation}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn open_head(name: &str) -> ExitCode {
    match hydra_core::open_head(Path::new("."), name) {
        Ok(opened) => {
            println!(
                "Opened Head {} at {}",
                opened.name,
                safe_path_label(&opened.path)
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn repair() -> ExitCode {
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
        show_repair_issue(issue);
    }
    if plan.abandoned_state_lock.is_some() {
        return repair_abandoned_state_lock();
    }
    if plan.missing_inventory.is_some() {
        return repair_missing_inventory(&plan);
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

fn show_repair_issue(issue: &hydra_core::RepairIssue) {
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

fn request_abandoned_lock_confirmation(
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> io::Result<bool> {
    write!(output, "Remove the abandoned Hydra state lock? [y/N] ")?;
    output.flush()?;
    let mut response = String::new();
    input.read_line(&mut response)?;
    Ok(matches!(
        response.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn request_inventory_recovery_confirmation(
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
    let mut response = String::new();
    input.read_line(&mut response)?;
    Ok(matches!(
        response.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn request_stale_inventory_confirmation(
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
    let mut response = String::new();
    input.read_line(&mut response)?;
    Ok(matches!(
        response.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn request_moved_worktree_confirmation(
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
    let mut response = String::new();
    input.read_line(&mut response)?;
    Ok(matches!(
        response.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn close_head(name: &str) -> ExitCode {
    match hydra_core::close_head(Path::new("."), name) {
        Ok(closed) => {
            match closed.outcome {
                hydra_core::CloseOutcome::Integrated {
                    target_commit,
                    strategy,
                    result,
                } => {
                    println!(
                        "Closed Head {} into {} at {}",
                        closed.name, closed.target_ref, target_commit
                    );
                    match strategy {
                        hydra_core::IntegrationStrategy::CheckoutFree => {
                            println!("Integration strategy: checkout-free");
                        }
                        hydra_core::IntegrationStrategy::TargetWorktree { path } => println!(
                            "Integration strategy: target worktree {}",
                            output::safe_path_label(&path)
                        ),
                    }
                    let result = match result {
                        hydra_core::IntegrationResult::AlreadyIntegrated => "already integrated",
                        hydra_core::IntegrationResult::FastForward => "fast-forward",
                        hydra_core::IntegrationResult::MergeCommit => "merge commit",
                    };
                    println!("Integration result: {result}");
                }
                hydra_core::CloseOutcome::CommandCompleted { removed, .. } if removed => {
                    println!(
                        "Close command completed for Head {}; Head removed",
                        closed.name
                    );
                }
                hydra_core::CloseOutcome::CommandCompleted { .. } => {
                    println!(
                        "Close command completed for Head {}; Head preserved",
                        closed.name
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn remove_head(name: &str, force: bool) -> ExitCode {
    match hydra_core::remove_head(
        Path::new("."),
        hydra_core::RemoveHeadOptions {
            name: name.to_owned(),
            force,
        },
    ) {
        Ok(removed) => {
            println!("Removed Head {}", removed.name);
            if let Some(branch) = removed.preserved_branch {
                println!("Preserved branch {branch} with unintegrated commits");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn create_head(name: &str, from: Option<&str>, target: Option<&str>) -> ExitCode {
    let progress_enabled = io::stderr().is_terminal();
    let create = |confirmed_full_copy, exclude_unsafe_overlay_symlinks| {
        hydra_core::create_head_with_progress(
            Path::new("."),
            hydra_core::CreateHeadOptions {
                name: name.to_owned(),
                from: from.map(str::to_owned),
                target: target.map(str::to_owned),
                confirmed_full_copy,
                exclude_unsafe_overlay_symlinks,
            },
            |progress| {
                if progress_enabled {
                    show_head_creation_progress(progress);
                }
            },
        )
    };

    let mut exclude_unsafe_overlay_symlinks = false;
    let first_result = create(false, false);
    let result = match first_result {
        Err(hydra_core::HeadError::UnsafeOverlaySymlinks { paths }) => {
            let stdin = io::stdin();
            let mut input = stdin.lock();
            let stdout = io::stdout();
            let mut output = stdout.lock();
            let confirmed =
                request_unsafe_symlink_exclusion(&mut input, &mut output, paths.as_slice());
            let confirmed = match confirmed {
                Ok(confirmed) => confirmed,
                Err(error) => {
                    eprintln!(
                        "error: failed to read unsafe-symlink exclusion confirmation: {error}"
                    );
                    return ExitCode::FAILURE;
                }
            };
            if !confirmed {
                eprintln!("error: Head creation cancelled");
                return ExitCode::FAILURE;
            }
            exclude_unsafe_overlay_symlinks = true;
            create(false, true)
        }
        result => result,
    };

    match result {
        Err(hydra_core::HeadError::OverlayFullCopyConfirmationRequired { files, bytes }) => {
            let stdin = io::stdin();
            let mut input = stdin.lock();
            let stdout = io::stdout();
            let mut output = stdout.lock();
            let confirmed = request_full_copy_confirmation(&mut input, &mut output, files, bytes);
            let confirmed = match confirmed {
                Ok(confirmed) => confirmed,
                Err(error) => {
                    eprintln!("error: failed to read full-copy confirmation: {error}");
                    return ExitCode::FAILURE;
                }
            };
            if !confirmed {
                eprintln!("error: Head creation cancelled");
                return ExitCode::FAILURE;
            }
            finish_head_creation(create(true, exclude_unsafe_overlay_symlinks))
        }
        result => finish_head_creation(result),
    }
}

fn show_head_creation_progress(progress: hydra_core::HeadCreationProgress) {
    let stderr = io::stderr();
    let _ = write_head_creation_progress(&mut stderr.lock(), progress);
}

fn write_head_creation_progress(
    output: &mut impl Write,
    progress: hydra_core::HeadCreationProgress,
) -> io::Result<()> {
    match progress {
        hydra_core::HeadCreationProgress::PlanningOverlays => {
            writeln!(output, "Planning overlays...")
        }
        hydra_core::HeadCreationProgress::MaterializingTrackedEntries { entries } => {
            writeln!(output, "Materializing {entries} tracked entries...")
        }
        hydra_core::HeadCreationProgress::MaterializingOverlayEntries { entries } => {
            writeln!(output, "Materializing {entries} overlay entries...")
        }
        _ => Ok(()),
    }
}

fn finish_head_creation(
    result: Result<hydra_core::CreatedHead, hydra_core::HeadError>,
) -> ExitCode {
    match result {
        Ok(head) => {
            if head.overlay_files > 0 {
                println!(
                    "Overlay: {} file(s), {} byte(s)",
                    head.overlay_files, head.overlay_bytes
                );
            }
            let stdout = io::stdout();
            let hyperlinks_enabled = stdout.is_terminal();
            if let Err(error) =
                write_created_head_path(&mut stdout.lock(), &head.path, hyperlinks_enabled)
            {
                eprintln!("error: failed to show the created Head path: {error}");
                return ExitCode::FAILURE;
            }
            match head.storage_backend {
                hydra_core::StorageBackend::CopyOnWrite => {
                    println!("Storage backend: copy-on-write");
                }
                hydra_core::StorageBackend::FullCopy => {
                    println!("Storage backend: full copy");
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn request_full_copy_confirmation(
    input: &mut impl BufRead,
    output: &mut impl Write,
    files: usize,
    bytes: u64,
) -> io::Result<bool> {
    write!(
        output,
        "Full copy required: {files} file(s), {bytes} byte(s)\nContinue? [y/N] "
    )?;
    output.flush()?;

    let mut response = String::new();
    input.read_line(&mut response)?;
    Ok(matches!(
        response.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn request_unsafe_symlink_exclusion(
    input: &mut impl BufRead,
    output: &mut impl Write,
    paths: &[PathBuf],
) -> io::Result<bool> {
    writeln!(output, "Unsafe overlay symlinks:")?;
    for path in paths {
        writeln!(output, "  {}", safe_path_label(path))?;
    }
    write!(output, "Exclude them and update .hydra.json? [y/N] ")?;
    output.flush()?;

    let mut response = String::new();
    input.read_line(&mut response)?;
    Ok(matches!(
        response.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn write_created_head_path(
    output: &mut impl Write,
    path: &Path,
    hyperlinks_enabled: bool,
) -> io::Result<()> {
    let label = safe_path_label(path);
    write!(output, "New Head successfully created at ")?;
    if hyperlinks_enabled && let Some(uri) = file_uri(path) {
        write!(output, "\u{1b}]8;;{uri}\u{1b}\\{label}\u{1b}]8;;\u{1b}\\")?;
    } else {
        write!(output, "{label}")?;
    }
    writeln!(output)
}

fn safe_path_label(path: &Path) -> String {
    output::safe_path_label(path)
}

#[cfg(unix)]
fn file_uri(path: &Path) -> Option<String> {
    use std::os::unix::ffi::OsStrExt;

    if !path.is_absolute() {
        return None;
    }
    let mut uri = String::from("file://");
    for byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            uri.push(char::from(*byte));
        } else {
            use std::fmt::Write as _;
            write!(uri, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    Some(uri)
}

#[cfg(not(unix))]
fn file_uri(_path: &Path) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, path::Path};

    use super::{
        Cli, request_full_copy_confirmation, write_created_head_path, write_head_creation_progress,
    };
    use clap::CommandFactory;

    #[test]
    fn clap_command_definition_is_internally_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn full_copy_confirmation_is_explicit_and_defaults_to_no() {
        let mut input = Cursor::new(b"\n");
        let mut output = Vec::new();

        let confirmed = request_full_copy_confirmation(&mut input, &mut output, 2, 13)
            .expect("prompt should be written and read");

        assert!(!confirmed);
        assert_eq!(
            String::from_utf8(output).expect("prompt should be UTF-8"),
            "Full copy required: 2 file(s), 13 byte(s)\nContinue? [y/N] "
        );
    }

    #[test]
    fn interactive_progress_describes_the_current_phase() {
        let mut output = Vec::new();

        write_head_creation_progress(
            &mut output,
            hydra_core::HeadCreationProgress::MaterializingTrackedEntries { entries: 1_840 },
        )
        .expect("tracked progress should be written");
        write_head_creation_progress(
            &mut output,
            hydra_core::HeadCreationProgress::MaterializingOverlayEntries { entries: 2_000 },
        )
        .expect("progress should be written");

        assert_eq!(
            String::from_utf8(output).expect("progress should be UTF-8"),
            concat!(
                "Materializing 1840 tracked entries...\n",
                "Materializing 2000 overlay entries...\n"
            )
        );
    }

    #[test]
    fn created_head_path_uses_an_osc_8_file_link_when_enabled() {
        let mut output = Vec::new();

        write_created_head_path(
            &mut output,
            Path::new("/projects/Hydra Demo.heads/payment#retry"),
            true,
        )
        .expect("success message should be written");

        assert_eq!(
            String::from_utf8(output).expect("message should be UTF-8"),
            "New Head successfully created at \u{1b}]8;;file:///projects/Hydra%20Demo.heads/payment%23retry\u{1b}\\/projects/Hydra Demo.heads/payment#retry\u{1b}]8;;\u{1b}\\\n"
        );
    }

    #[test]
    fn created_head_path_remains_plain_when_hyperlinks_are_disabled() {
        let mut output = Vec::new();

        write_created_head_path(
            &mut output,
            Path::new("/projects/demo.heads/payment"),
            false,
        )
        .expect("success message should be written");

        assert_eq!(
            String::from_utf8(output).expect("message should be UTF-8"),
            "New Head successfully created at /projects/demo.heads/payment\n"
        );
    }

    #[test]
    fn created_head_path_encodes_control_characters_in_uri_and_visible_text() {
        let mut output = Vec::new();

        write_created_head_path(
            &mut output,
            Path::new("/projects/demo\nheads/payment"),
            true,
        )
        .expect("success message should be written");

        assert_eq!(
            String::from_utf8(output).expect("message should be UTF-8"),
            "New Head successfully created at \u{1b}]8;;file:///projects/demo%0Aheads/payment\u{1b}\\/projects/demo\\nheads/payment\u{1b}]8;;\u{1b}\\\n"
        );
    }
}

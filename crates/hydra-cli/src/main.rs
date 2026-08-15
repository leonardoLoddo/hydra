use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{CommandFactory as _, Parser, Subcommand, ValueEnum};
use clap_complete::{
    CompleteEnv,
    engine::{ArgValueCompleter, CompletionCandidate},
    env::{Bash, EnvCompleter, Fish, Shells, Zsh},
};

mod head_create;
mod inspection;
mod output;
mod repair;

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
        Command::Repair => repair::run(),
        Command::Doctor {
            command: DoctorCommand::Storage,
        } => doctor_storage(),
        Command::Completions { shell } => print_completions(shell),
        Command::Head {
            command: HeadCommand::Create { name, from, target },
        } => head_create::run(&name, from.as_deref(), target.as_deref()),
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

fn safe_path_label(path: &Path) -> String {
    output::safe_path_label(path)
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn clap_command_definition_is_internally_consistent() {
        Cli::command().debug_assert();
    }
}

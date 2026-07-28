use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "hydra",
    version,
    about = "Git-native workspace manager for isolated development Heads"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize Hydra for a Git repository
    Init {
        /// Repository path
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

fn main() -> ExitCode {
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
    }
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

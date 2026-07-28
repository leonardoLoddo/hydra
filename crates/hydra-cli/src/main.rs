use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

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
    /// Manage isolated development Heads
    Head {
        #[command(subcommand)]
        command: HeadCommand,
    },
}

#[derive(Subcommand)]
enum HeadCommand {
    /// Create an isolated Head
    Create {
        /// Logical Head name
        name: String,
        /// Git ref or commit used as the immutable base
        #[arg(long)]
        from: Option<String>,
        /// Local branch intended as the integration target
        #[arg(long)]
        target: Option<String>,
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
        Command::Head {
            command: HeadCommand::Create { name, from, target },
        } => create_head(&name, from.as_deref(), target.as_deref()),
    }
}

fn create_head(name: &str, from: Option<&str>, target: Option<&str>) -> ExitCode {
    let create = |confirmed_overlays| {
        hydra_core::create_head(
            Path::new("."),
            hydra_core::CreateHeadOptions {
                name: name.to_owned(),
                from: from.map(str::to_owned),
                target: target.map(str::to_owned),
                confirmed_overlays,
            },
        )
    };

    match create(false) {
        Err(hydra_core::HeadError::OverlayConfirmationRequired { files, bytes }) => {
            println!("Overlay: {files} file(s), {bytes} byte(s)");
            print!("Copy these overlay files? [y/N] ");
            if let Err(error) = io::stdout().flush() {
                eprintln!("error: failed to show overlay confirmation: {error}");
                return ExitCode::FAILURE;
            }

            let mut response = String::new();
            if let Err(error) = io::stdin().read_line(&mut response) {
                eprintln!("error: failed to read overlay confirmation: {error}");
                return ExitCode::FAILURE;
            }
            if !matches!(response.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
                eprintln!("error: Head creation cancelled");
                return ExitCode::FAILURE;
            }
            finish_head_creation(create(true))
        }
        result => finish_head_creation(result),
    }
}

fn finish_head_creation(
    result: Result<hydra_core::CreatedHead, hydra_core::HeadError>,
) -> ExitCode {
    match result {
        Ok(head) => {
            println!("Created Head {} at {}", head.name, head.path.display());
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

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn clap_command_definition_is_internally_consistent() {
        Cli::command().debug_assert();
    }
}

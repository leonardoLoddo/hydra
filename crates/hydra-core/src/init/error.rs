use std::{error::Error, fmt, path::PathBuf};

#[derive(Debug)]
pub enum InitError {
    GitUnavailable(std::io::Error),
    GitCommandFailed {
        operation: &'static str,
        status: Option<i32>,
        stderr: String,
    },
    InvalidGitOutput(&'static str),
    UnsupportedRepositoryPath(PathBuf),
    UnsupportedRepositoryName(PathBuf),
    AlreadyInitialized(PathBuf),
    HeadsDirectoryExists(PathBuf),
    LocalStateExists(PathBuf),
    StateDirectoryExists(PathBuf),
    UnsafeStateDirectory(PathBuf),
    ExistingInstallationIncomplete(PathBuf),
    ExistingOwnershipMismatch(PathBuf),
    ExistingInstallationChanged(PathBuf),
    ExistingHeadsRequireConfiguration(PathBuf),
    InvalidLocalMetadata {
        kind: &'static str,
        path: PathBuf,
        source: serde_json::Error,
    },
    UnsupportedLocalMetadataVersion {
        kind: &'static str,
        version: u32,
    },
    SerializeConfiguration(serde_json::Error),
    FileSystem {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    RollbackFailed {
        original: Box<InitError>,
        cleanup_failures: Vec<CleanupFailure>,
    },
    CleanupFailed {
        operation: &'static str,
        cleanup_failures: Vec<CleanupFailure>,
    },
    InvalidStorageProbe(PathBuf),
}

#[derive(Debug)]
pub struct CleanupFailure {
    pub path: PathBuf,
    pub source: std::io::Error,
}

impl fmt::Display for InitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitUnavailable(error) => write!(formatter, "could not run Git: {error}"),
            Self::GitCommandFailed {
                operation,
                status,
                stderr,
            } => format_git_failure(formatter, operation, *status, stderr),
            Self::InvalidGitOutput(field) => {
                write!(formatter, "Git returned an invalid {field}")
            }
            Self::UnsupportedRepositoryPath(path) => write!(
                formatter,
                "cannot derive a sibling Heads directory for {}",
                path.display()
            ),
            Self::UnsupportedRepositoryName(path) => write!(
                formatter,
                "repository name {} is not valid UTF-8 and cannot be stored losslessly",
                path.display()
            ),
            Self::AlreadyInitialized(path) => {
                write!(
                    formatter,
                    "Hydra is already initialized at {}",
                    path.display()
                )
            }
            Self::HeadsDirectoryExists(path) => write!(
                formatter,
                "Heads directory {} already exists and was not created by this initialization",
                path.display()
            ),
            Self::LocalStateExists(path) => write!(
                formatter,
                "local Hydra state already exists at {}",
                path.display()
            ),
            Self::StateDirectoryExists(path) => write!(
                formatter,
                "local Hydra state directory {} already exists and its ownership is unknown",
                path.display()
            ),
            Self::UnsafeStateDirectory(path) => write!(
                formatter,
                "local Hydra state directory {} is not a real directory",
                path.display()
            ),
            error @ (Self::ExistingInstallationIncomplete(_)
            | Self::ExistingOwnershipMismatch(_)
            | Self::ExistingInstallationChanged(_)
            | Self::ExistingHeadsRequireConfiguration(_)
            | Self::InvalidLocalMetadata { .. }
            | Self::UnsupportedLocalMetadataVersion { .. }) => {
                format_existing_installation_error(formatter, error)
            }
            Self::SerializeConfiguration(error) => {
                write!(
                    formatter,
                    "could not serialize Hydra configuration: {error}"
                )
            }
            Self::FileSystem {
                action,
                path,
                source,
            } => write!(formatter, "could not {action} {}: {source}", path.display()),
            Self::RollbackFailed {
                original,
                cleanup_failures,
            } => {
                write!(
                    formatter,
                    "{original}; rollback left {} artifact(s): ",
                    cleanup_failures.len()
                )?;
                format_cleanup_failures(formatter, cleanup_failures)
            }
            Self::CleanupFailed {
                operation,
                cleanup_failures,
            } => {
                write!(
                    formatter,
                    "{operation} left {} temporary artifact(s): ",
                    cleanup_failures.len()
                )?;
                format_cleanup_failures(formatter, cleanup_failures)
            }
            Self::InvalidStorageProbe(path) => write!(
                formatter,
                "storage probe produced unexpected content at {}",
                path.display()
            ),
        }
    }
}

fn format_git_failure(
    formatter: &mut fmt::Formatter<'_>,
    operation: &str,
    status: Option<i32>,
    stderr: &str,
) -> fmt::Result {
    let status = status.map_or_else(|| "unknown".to_owned(), |code| code.to_string());
    let stderr = stderr.trim_end_matches(['\r', '\n']);
    write!(
        formatter,
        "Git failed while resolving {operation} with status {status}"
    )?;
    if !stderr.is_empty() {
        write!(formatter, ": {stderr}")?;
    }
    Ok(())
}

fn format_existing_installation_error(
    formatter: &mut fmt::Formatter<'_>,
    error: &InitError,
) -> fmt::Result {
    match error {
        InitError::ExistingInstallationIncomplete(path) => write!(
            formatter,
            "existing Hydra installation is incomplete at {}",
            path.display()
        ),
        InitError::ExistingOwnershipMismatch(path) => write!(
            formatter,
            "existing Hydra ownership metadata do not match at {}",
            path.display()
        ),
        InitError::ExistingInstallationChanged(path) => write!(
            formatter,
            "existing Hydra installation changed during validation at {}",
            path.display()
        ),
        InitError::ExistingHeadsRequireConfiguration(path) => write!(
            formatter,
            "cannot safely reconstruct configuration for existing Heads recorded at {}",
            path.display()
        ),
        InitError::InvalidLocalMetadata { kind, path, source } => {
            write!(formatter, "invalid {kind} at {}: {source}", path.display())
        }
        InitError::UnsupportedLocalMetadataVersion { kind, version } => {
            write!(formatter, "unsupported {kind} version {version}")
        }
        _ => unreachable!("only existing-installation errors are routed here"),
    }
}

fn format_cleanup_failures(
    formatter: &mut fmt::Formatter<'_>,
    cleanup_failures: &[CleanupFailure],
) -> fmt::Result {
    for (index, failure) in cleanup_failures.iter().enumerate() {
        if index > 0 {
            write!(formatter, ", ")?;
        }
        write!(formatter, "{} ({})", failure.path.display(), failure.source)?;
    }
    Ok(())
}

impl Error for InitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::GitUnavailable(error) => Some(error),
            Self::SerializeConfiguration(error) => Some(error),
            Self::InvalidLocalMetadata { source, .. } => Some(source),
            Self::FileSystem { source, .. } => Some(source),
            Self::RollbackFailed { original, .. } => Some(original.as_ref()),
            Self::CleanupFailed {
                cleanup_failures, ..
            } => cleanup_failures
                .first()
                .map(|failure| &failure.source as &(dyn Error + 'static)),
            Self::GitCommandFailed { .. }
            | Self::InvalidGitOutput(_)
            | Self::UnsupportedRepositoryPath(_)
            | Self::UnsupportedRepositoryName(_)
            | Self::AlreadyInitialized(_)
            | Self::HeadsDirectoryExists(_)
            | Self::LocalStateExists(_)
            | Self::StateDirectoryExists(_)
            | Self::UnsafeStateDirectory(_)
            | Self::ExistingInstallationIncomplete(_)
            | Self::ExistingOwnershipMismatch(_)
            | Self::ExistingInstallationChanged(_)
            | Self::ExistingHeadsRequireConfiguration(_)
            | Self::UnsupportedLocalMetadataVersion { .. }
            | Self::InvalidStorageProbe(_) => None,
        }
    }
}

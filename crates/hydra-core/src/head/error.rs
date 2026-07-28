use std::{error::Error, fmt, path::PathBuf};

#[derive(Debug)]
pub enum HeadError {
    GitUnavailable(std::io::Error),
    GitCommandFailed {
        operation: &'static str,
        status: Option<i32>,
        stderr: String,
    },
    ProjectNotInitialized(PathBuf),
    InvalidConfiguration {
        path: PathBuf,
        source: serde_json::Error,
    },
    InvalidState {
        path: PathBuf,
        source: serde_json::Error,
    },
    UnsupportedConfigurationVersion(u32),
    UnsupportedStateVersion(u32),
    InvalidName(String),
    HeadAlreadyExists(String),
    DestinationExists(PathBuf),
    BranchAlreadyExists(String),
    InvalidRef(String),
    TargetRequired,
    OverlayConfirmationRequired {
        files: usize,
        bytes: u64,
    },
    OverlayRules(String),
    UnsafeOverlayPath(PathBuf),
    OverlayOverwritesTracked(PathBuf),
    OverlayChanged(PathBuf),
    UnsafeProjectFile(PathBuf),
    UnsafeHeadsDirectory(PathBuf),
    UnsupportedTrackedEntry {
        mode: String,
        path: PathBuf,
    },
    InvalidGitOutput(&'static str),
    ConcurrentStateChange(PathBuf),
    StateLockExists(PathBuf),
    SerializeState(serde_json::Error),
    Timestamp(time::error::Format),
    FileSystem {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    RollbackFailed {
        original: Box<HeadError>,
        failures: Vec<String>,
    },
    HeadCommittedWithCleanupFailure(Box<HeadError>),
}

impl fmt::Display for HeadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitUnavailable(error) => write!(formatter, "could not run Git: {error}"),
            Self::GitCommandFailed {
                operation,
                status,
                stderr,
            } => display_git_failure(formatter, operation, *status, stderr),
            Self::ProjectNotInitialized(path) => display_not_initialized(formatter, path),
            Self::InvalidConfiguration { path, source } => {
                display_invalid_json(formatter, "configuration", path, source)
            }
            Self::InvalidState { path, source } => {
                display_invalid_json(formatter, "state", path, source)
            }
            Self::UnsupportedConfigurationVersion(version) => {
                display_unsupported_version(formatter, "configuration", *version)
            }
            Self::UnsupportedStateVersion(version) => {
                display_unsupported_version(formatter, "state", *version)
            }
            Self::InvalidName(name) => write!(formatter, "invalid Head name {name:?}"),
            Self::HeadAlreadyExists(name) => write!(formatter, "Head {name:?} already exists"),
            Self::DestinationExists(path) => {
                write!(
                    formatter,
                    "Head destination {} already exists",
                    path.display()
                )
            }
            Self::BranchAlreadyExists(branch) => {
                write!(formatter, "Head branch {branch:?} already exists")
            }
            Self::InvalidRef(reference) => {
                write!(
                    formatter,
                    "Git ref {reference:?} does not resolve as required"
                )
            }
            Self::TargetRequired => write!(
                formatter,
                "--target is required when --from does not resolve to a local branch"
            ),
            Self::OverlayConfirmationRequired { files, bytes } => write!(
                formatter,
                "copying {files} overlay file(s) ({bytes} byte(s)) requires confirmation"
            ),
            Self::OverlayRules(error) => write!(formatter, "overlay rules are invalid: {error}"),
            Self::UnsafeOverlayPath(path) => {
                write!(formatter, "overlay path {} is unsafe", path.display())
            }
            Self::OverlayOverwritesTracked(path) => write!(
                formatter,
                "overlay path {} would overwrite a tracked file",
                path.display()
            ),
            Self::OverlayChanged(path) => write!(
                formatter,
                "overlay source {} changed during materialization",
                path.display()
            ),
            Self::UnsafeProjectFile(path) => {
                write!(
                    formatter,
                    "{} must be a regular file, not a symlink",
                    path.display()
                )
            }
            Self::UnsafeHeadsDirectory(path) => {
                write!(formatter, "Heads directory {} is unsafe", path.display())
            }
            Self::UnsupportedTrackedEntry { mode, path } => write!(
                formatter,
                "tracked entry {} has unsupported Git mode {mode}",
                path.display()
            ),
            Self::InvalidGitOutput(field) => write!(formatter, "Git returned an invalid {field}"),
            Self::ConcurrentStateChange(path) => write!(
                formatter,
                "Hydra state {} changed while the Head was being created",
                path.display()
            ),
            Self::StateLockExists(path) => write!(
                formatter,
                "another Hydra state operation owns lock {}",
                path.display()
            ),
            Self::SerializeState(error) => write!(formatter, "could not serialize state: {error}"),
            Self::Timestamp(error) => write!(formatter, "could not format creation time: {error}"),
            Self::FileSystem {
                action,
                path,
                source,
            } => write!(formatter, "could not {action} {}: {source}", path.display()),
            Self::RollbackFailed { original, failures } => {
                display_rollback_failure(formatter, original, failures)
            }
            Self::HeadCommittedWithCleanupFailure(error) => {
                display_committed_cleanup_failure(formatter, error)
            }
        }
    }
}

fn display_git_failure(
    formatter: &mut fmt::Formatter<'_>,
    operation: &str,
    status: Option<i32>,
    stderr: &str,
) -> fmt::Result {
    let status = status.map_or_else(|| "unknown".to_owned(), |code| code.to_string());
    write!(
        formatter,
        "Git failed while {operation} with status {status}"
    )?;
    let stderr = stderr.trim_end_matches(['\r', '\n']);
    if !stderr.is_empty() {
        write!(formatter, ": {stderr}")?;
    }
    Ok(())
}

fn display_invalid_json(
    formatter: &mut fmt::Formatter<'_>,
    kind: &str,
    path: &std::path::Path,
    source: &serde_json::Error,
) -> fmt::Result {
    write!(
        formatter,
        "Hydra {kind} {} is invalid: {source}",
        path.display()
    )
}

fn display_not_initialized(
    formatter: &mut fmt::Formatter<'_>,
    path: &std::path::Path,
) -> fmt::Result {
    write!(formatter, "Hydra is not initialized at {}", path.display())
}

fn display_unsupported_version(
    formatter: &mut fmt::Formatter<'_>,
    kind: &str,
    version: u32,
) -> fmt::Result {
    write!(formatter, "Hydra {kind} version {version} is not supported")
}

fn display_rollback_failure(
    formatter: &mut fmt::Formatter<'_>,
    original: &HeadError,
    failures: &[String],
) -> fmt::Result {
    write!(
        formatter,
        "{original}; rollback also failed: {}",
        failures.join(", ")
    )
}

fn display_committed_cleanup_failure(
    formatter: &mut fmt::Formatter<'_>,
    error: &HeadError,
) -> fmt::Result {
    write!(formatter, "Head was committed, but cleanup failed: {error}")
}

impl Error for HeadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::GitUnavailable(error) => Some(error),
            Self::InvalidConfiguration { source, .. } | Self::InvalidState { source, .. } => {
                Some(source)
            }
            Self::SerializeState(error) => Some(error),
            Self::Timestamp(error) => Some(error),
            Self::FileSystem { source, .. } => Some(source),
            Self::RollbackFailed { original, .. }
            | Self::HeadCommittedWithCleanupFailure(original) => Some(original.as_ref()),
            _ => None,
        }
    }
}

impl HeadError {
    pub(super) fn head_was_committed(&self) -> bool {
        matches!(self, Self::HeadCommittedWithCleanupFailure(_))
    }
}

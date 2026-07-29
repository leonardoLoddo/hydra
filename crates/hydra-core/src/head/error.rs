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
    InvalidLocalMetadata {
        kind: &'static str,
        path: PathBuf,
        source: serde_json::Error,
    },
    UnsupportedConfigurationVersion(u32),
    UnsupportedStateVersion(u32),
    UnsupportedLocalMetadataVersion {
        kind: &'static str,
        version: u32,
    },
    LocalIdentityMismatch(PathBuf),
    DirectoryPolicyMismatch(PathBuf),
    InvalidName(String),
    HeadAlreadyExists(String),
    HeadNotFound(String),
    OpenCommandNotConfigured,
    InvalidOpenCommand(String),
    OpenCommandUnavailable {
        program: String,
        source: std::io::Error,
    },
    OpenCommandFailed {
        program: String,
        status: Option<i32>,
    },
    HeadOpenInconsistent {
        name: String,
        reason: &'static str,
    },
    HeadHasUncommittedChanges(String),
    HeadHasUnintegratedCommits {
        head_ref: String,
        target_ref: String,
    },
    HeadRemovalInconsistent {
        name: String,
        reason: &'static str,
    },
    HeadRemovalIncomplete {
        name: String,
        preserved_branch: String,
        source: Box<HeadError>,
    },
    HeadCloseInconsistent {
        name: String,
        reason: String,
    },
    HeadCloseTargetCheckedOut {
        target_ref: String,
        path: PathBuf,
    },
    HeadCloseConflict {
        name: String,
        target_ref: String,
    },
    HeadIntegratedButRemovalFailed {
        name: String,
        target_ref: String,
        target_commit: String,
        source: Box<HeadError>,
    },
    DestinationExists(PathBuf),
    BranchAlreadyExists(String),
    InvalidRef(String),
    TargetRequired,
    OverlayFullCopyConfirmationRequired {
        files: usize,
        bytes: u64,
    },
    UnsafeOverlaySymlinks {
        paths: Vec<PathBuf>,
    },
    OverlayRules(String),
    UnsafeOverlayPath(PathBuf),
    UnsafeHeadPath(PathBuf),
    OverlayOverwritesTracked(PathBuf),
    OverlayChanged(PathBuf),
    UnsafeProjectFile(PathBuf),
    UnsafeHeadsDirectory(PathBuf),
    UnsupportedTrackedEntry {
        mode: String,
        path: PathBuf,
    },
    InvalidGitOutput(&'static str),
    ConcurrentConfigurationChange(PathBuf),
    ConcurrentStateChange(PathBuf),
    StateLockExists(PathBuf),
    SerializeConfiguration(serde_json::Error),
    SerializeState(serde_json::Error),
    Timestamp(time::error::Format),
    FileSystem {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    ConfigurationCommittedWithCleanupFailure(Box<HeadError>),
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
                display_invalid_configuration(formatter, path, source)
            }
            Self::InvalidState { path, source } => display_invalid_state(formatter, path, source),
            Self::InvalidLocalMetadata { kind, path, source } => {
                display_invalid_json(formatter, kind, path, source)
            }
            Self::UnsupportedConfigurationVersion(version) => {
                display_unsupported_configuration(formatter, *version)
            }
            Self::UnsupportedStateVersion(version) => {
                display_unsupported_state(formatter, *version)
            }
            Self::UnsupportedLocalMetadataVersion { kind, version } => {
                display_unsupported_version(formatter, kind, *version)
            }
            Self::LocalIdentityMismatch(path) => display_identity_mismatch(formatter, path),
            Self::DirectoryPolicyMismatch(path) => display_policy_mismatch(formatter, path),
            Self::InvalidName(name) => write!(formatter, "invalid Head name {name:?}"),
            Self::HeadAlreadyExists(name) => write!(formatter, "Head {name:?} already exists"),
            Self::HeadNotFound(name) => write!(formatter, "Head {name:?} does not exist"),
            Self::OpenCommandNotConfigured
            | Self::InvalidOpenCommand(_)
            | Self::OpenCommandUnavailable { .. }
            | Self::OpenCommandFailed { .. }
            | Self::HeadOpenInconsistent { .. } => display_open_failure(formatter, self),
            Self::HeadHasUncommittedChanges(_)
            | Self::HeadHasUnintegratedCommits { .. }
            | Self::HeadRemovalInconsistent { .. }
            | Self::HeadRemovalIncomplete { .. } => display_removal_failure(formatter, self),
            Self::HeadCloseInconsistent { .. }
            | Self::HeadCloseTargetCheckedOut { .. }
            | Self::HeadCloseConflict { .. }
            | Self::HeadIntegratedButRemovalFailed { .. } => display_close_failure(formatter, self),
            Self::DestinationExists(path) => display_destination_exists(formatter, path),
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
            Self::OverlayFullCopyConfirmationRequired { .. }
            | Self::UnsafeOverlaySymlinks { .. }
            | Self::OverlayRules(_)
            | Self::UnsafeOverlayPath(_)
            | Self::OverlayOverwritesTracked(_)
            | Self::OverlayChanged(_) => display_overlay_failure(formatter, self),
            Self::UnsafeHeadPath(path) => display_unsafe_path(formatter, "recorded Head", path),
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
            Self::ConcurrentConfigurationChange(_)
            | Self::ConcurrentStateChange(_)
            | Self::StateLockExists(_)
            | Self::SerializeConfiguration(_)
            | Self::SerializeState(_)
            | Self::ConfigurationCommittedWithCleanupFailure(_)
            | Self::HeadCommittedWithCleanupFailure(_) => {
                display_persistence_failure(formatter, self)
            }
            Self::Timestamp(error) => write!(formatter, "could not format creation time: {error}"),
            Self::FileSystem {
                action,
                path,
                source,
            } => write!(formatter, "could not {action} {}: {source}", path.display()),
            Self::RollbackFailed { original, failures } => {
                display_rollback_failure(formatter, original, failures)
            }
        }
    }
}

fn display_overlay_failure(formatter: &mut fmt::Formatter<'_>, error: &HeadError) -> fmt::Result {
    match error {
        HeadError::OverlayFullCopyConfirmationRequired { files, bytes } => write!(
            formatter,
            "copying {files} overlay file(s) ({bytes} byte(s)) requires confirmation"
        ),
        HeadError::UnsafeOverlaySymlinks { paths } => write!(
            formatter,
            "{} unsafe overlay symlink(s) require exclusion",
            paths.len()
        ),
        HeadError::OverlayRules(error) => write!(formatter, "overlay rules are invalid: {error}"),
        HeadError::UnsafeOverlayPath(path) => display_unsafe_path(formatter, "overlay", path),
        HeadError::OverlayOverwritesTracked(path) => write!(
            formatter,
            "overlay path {} would overwrite a tracked file",
            path.display()
        ),
        HeadError::OverlayChanged(path) => write!(
            formatter,
            "overlay source {} changed during materialization",
            path.display()
        ),
        _ => unreachable!("caller selects overlay failures"),
    }
}

fn display_open_failure(formatter: &mut fmt::Formatter<'_>, error: &HeadError) -> fmt::Result {
    match error {
        HeadError::OpenCommandNotConfigured => {
            write!(formatter, "open command is not configured in .hydra.json")
        }
        HeadError::InvalidOpenCommand(reason) => {
            write!(formatter, "open command is invalid: {reason}")
        }
        HeadError::OpenCommandUnavailable { program, source } => {
            write!(
                formatter,
                "could not start open command {program:?}: {source}"
            )
        }
        HeadError::OpenCommandFailed { program, status } => {
            let status = status.map_or_else(|| "unknown".to_owned(), |code| code.to_string());
            write!(
                formatter,
                "open command failed: {program:?} exited with status {status}"
            )
        }
        HeadError::HeadOpenInconsistent { name, reason } => {
            write!(formatter, "Head {name:?} cannot be opened safely: {reason}")
        }
        _ => unreachable!("caller selects Head-open failures"),
    }
}

fn display_close_failure(formatter: &mut fmt::Formatter<'_>, error: &HeadError) -> fmt::Result {
    match error {
        HeadError::HeadCloseInconsistent { name, reason } => {
            write!(formatter, "Head {name:?} cannot be closed safely: {reason}")
        }
        HeadError::HeadCloseTargetCheckedOut { target_ref, path } => write!(
            formatter,
            "target {target_ref} is checked out at {}; switch that worktree before closing",
            path.display()
        ),
        HeadError::HeadCloseConflict { name, target_ref } => write!(
            formatter,
            "Head {name:?} conflicts with {target_ref}; target and Head were preserved"
        ),
        HeadError::HeadIntegratedButRemovalFailed {
            name,
            target_ref,
            target_commit,
            source,
        } => write!(
            formatter,
            "Head {name:?} was integrated into {target_ref} at {target_commit}, but protected removal failed: {source}"
        ),
        _ => unreachable!("caller selects Head-close failures"),
    }
}

fn display_removal_failure(formatter: &mut fmt::Formatter<'_>, error: &HeadError) -> fmt::Result {
    match error {
        HeadError::HeadHasUncommittedChanges(name) => write!(
            formatter,
            "Head {name:?} has uncommitted changes; use --force to discard them"
        ),
        HeadError::HeadHasUnintegratedCommits {
            head_ref,
            target_ref,
        } => write!(
            formatter,
            "Head branch {head_ref} has commits not integrated into {target_ref}"
        ),
        HeadError::HeadRemovalInconsistent { name, reason } => {
            write!(
                formatter,
                "Head {name:?} cannot be removed safely: {reason}"
            )
        }
        HeadError::HeadRemovalIncomplete {
            name,
            preserved_branch,
            source,
        } => write!(
            formatter,
            "Head {name:?} removal is incomplete; branch {preserved_branch} was preserved: {source}"
        ),
        _ => unreachable!("caller selects Head-removal failures"),
    }
}

fn display_persistence_failure(
    formatter: &mut fmt::Formatter<'_>,
    error: &HeadError,
) -> fmt::Result {
    match error {
        HeadError::ConcurrentConfigurationChange(path) => write!(
            formatter,
            "Hydra configuration {} changed while exclusions were being saved",
            path.display()
        ),
        HeadError::ConcurrentStateChange(path) => write!(
            formatter,
            "Hydra state {} changed while the Head was being created",
            path.display()
        ),
        HeadError::StateLockExists(path) => write!(
            formatter,
            "another Hydra state operation owns lock {}",
            path.display()
        ),
        HeadError::SerializeConfiguration(error) => {
            write!(formatter, "could not serialize configuration: {error}")
        }
        HeadError::SerializeState(error) => write!(formatter, "could not serialize state: {error}"),
        HeadError::ConfigurationCommittedWithCleanupFailure(error) => write!(
            formatter,
            ".hydra.json was updated, but cleanup failed: {error}"
        ),
        HeadError::HeadCommittedWithCleanupFailure(error) => {
            display_committed_cleanup_failure(formatter, error)
        }
        _ => unreachable!("caller selects persistence failures"),
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

fn display_unsafe_path(
    formatter: &mut fmt::Formatter<'_>,
    kind: &str,
    path: &std::path::Path,
) -> fmt::Result {
    write!(formatter, "{kind} path {} is unsafe", path.display())
}

fn display_invalid_configuration(
    formatter: &mut fmt::Formatter<'_>,
    path: &std::path::Path,
    source: &serde_json::Error,
) -> fmt::Result {
    display_invalid_json(formatter, "configuration", path, source)
}

fn display_invalid_state(
    formatter: &mut fmt::Formatter<'_>,
    path: &std::path::Path,
    source: &serde_json::Error,
) -> fmt::Result {
    display_invalid_json(formatter, "state", path, source)
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

fn display_unsupported_configuration(
    formatter: &mut fmt::Formatter<'_>,
    version: u32,
) -> fmt::Result {
    display_unsupported_version(formatter, "configuration", version)
}

fn display_unsupported_state(formatter: &mut fmt::Formatter<'_>, version: u32) -> fmt::Result {
    display_unsupported_version(formatter, "state", version)
}

fn display_identity_mismatch(
    formatter: &mut fmt::Formatter<'_>,
    path: &std::path::Path,
) -> fmt::Result {
    write!(
        formatter,
        "Hydra directory ownership does not match the local project at {}",
        path.display()
    )
}

fn display_destination_exists(
    formatter: &mut fmt::Formatter<'_>,
    path: &std::path::Path,
) -> fmt::Result {
    write!(
        formatter,
        "Head destination {} already exists",
        path.display()
    )
}

fn display_policy_mismatch(
    formatter: &mut fmt::Formatter<'_>,
    path: &std::path::Path,
) -> fmt::Result {
    write!(
        formatter,
        "Heads directory {} does not match the versioned directory policy",
        path.display()
    )
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
            Self::InvalidConfiguration { source, .. }
            | Self::InvalidState { source, .. }
            | Self::InvalidLocalMetadata { source, .. } => Some(source),
            Self::SerializeConfiguration(error) | Self::SerializeState(error) => Some(error),
            Self::Timestamp(error) => Some(error),
            Self::FileSystem { source, .. } | Self::OpenCommandUnavailable { source, .. } => {
                Some(source)
            }
            Self::ConfigurationCommittedWithCleanupFailure(original)
            | Self::RollbackFailed { original, .. }
            | Self::HeadCommittedWithCleanupFailure(original)
            | Self::HeadRemovalIncomplete {
                source: original, ..
            }
            | Self::HeadIntegratedButRemovalFailed {
                source: original, ..
            } => Some(original.as_ref()),
            _ => None,
        }
    }
}

impl HeadError {
    pub(super) fn head_was_committed(&self) -> bool {
        matches!(self, Self::HeadCommittedWithCleanupFailure(_))
    }
}

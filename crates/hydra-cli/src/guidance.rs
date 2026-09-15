pub(super) fn report_init_error(error: &hydra_core::InitError) {
    eprintln!("error: {error}");
    eprintln!("next: {}", init_next_step(error));
}

pub(super) fn report_head_error(error: &hydra_core::HeadError) {
    eprintln!("error: {error}");
    eprintln!("next: {}", head_next_step(error));
}

pub(super) fn report_repair_error(error: &hydra_core::HeadError) {
    eprintln!("error: {error}");
    match error {
        hydra_core::HeadError::StateLockExists(_) => {
            eprintln!("next: Wait for the active operation to finish, then rerun `hydra repair`.");
        }
        _ => {
            eprintln!(
                "next: Preserve the reported state, resolve the validation, Git, or filesystem error, then rerun `hydra repair`."
            );
        }
    }
}

pub(super) fn report_doctor_error(error: &hydra_core::DoctorError) {
    eprintln!("error: {error}");
    let next = match error {
        hydra_core::DoctorError::Project(error) => head_next_step(error),
        hydra_core::DoctorError::Probe(error) => init_next_step(error),
        hydra_core::DoctorError::FileSystem { .. }
        | hydra_core::DoctorError::ProbeCleanupFailed { .. } => {
            "Preserve any reported temporary path, resolve the filesystem problem, then rerun `hydra doctor storage`."
        }
    };
    eprintln!("next: {next}");
}

pub(super) fn report_skill_error(error: &crate::skill::SkillError) {
    eprintln!("error: {error}");
    eprintln!("next: {}", skill_next_step(error));
}

pub(super) fn report_input_error(context: &str, error: &std::io::Error) {
    eprintln!("error: failed to read {context}: {error}");
    eprintln!("next: Retry in an interactive terminal and answer the confirmation prompt.");
}

fn init_next_step(error: &hydra_core::InitError) -> &'static str {
    use hydra_core::InitError;

    match error {
        InitError::AlreadyInitialized(_) => "Run `hydra status` to inspect the existing project.",
        InitError::InitializationInProgress(_) => {
            "Wait for the active initialization to finish, then rerun the same `hydra init [PATH]`."
        }
        InitError::InterruptedInitializationMismatch(_)
        | InitError::ExistingInstallationIncomplete(_)
        | InitError::ExistingOwnershipMismatch(_)
        | InitError::ExistingInstallationChanged(_)
        | InitError::ExistingHeadsRequireConfiguration(_)
        | InitError::InvalidLocalMetadata { .. }
        | InitError::UnsupportedLocalMetadataVersion { .. } => {
            "Preserve the reported paths; do not edit local Hydra metadata. Diagnose the mismatch before rerunning `hydra init`."
        }
        InitError::HeadsDirectoryExists(_)
        | InitError::LocalStateExists(_)
        | InitError::StateDirectoryExists(_)
        | InitError::UnsafeStateDirectory(_) => {
            "Inspect and preserve the existing path; initialize only after its ownership and intended use are known."
        }
        InitError::GitUnavailable(_)
        | InitError::GitCommandFailed { .. }
        | InitError::InvalidGitOutput(_) => {
            "Verify Git is installed and the target is a healthy Git repository, then retry."
        }
        InitError::UnsupportedRepositoryPath(_) | InitError::UnsupportedRepositoryName(_) => {
            "Move or rename the repository to a supported path, then rerun `hydra init`."
        }
        InitError::SerializeConfiguration(_) => {
            "Retry with the current Hydra release; if it repeats, preserve the error and report it."
        }
        InitError::FileSystem { .. }
        | InitError::RollbackFailed { .. }
        | InitError::CleanupFailed { .. }
        | InitError::InvalidStorageProbe(_) => {
            "Resolve the reported filesystem or storage problem, preserve any named residue, then rerun the same `hydra init [PATH]`."
        }
    }
}

#[allow(clippy::too_many_lines)]
fn head_next_step(error: &hydra_core::HeadError) -> &'static str {
    use hydra_core::HeadError;

    match error {
        HeadError::ProjectNotInitialized(_) => {
            "Run `hydra init [PATH]` in the intended Git repository first."
        }
        HeadError::HeadNotFound(_) => "Run `hydra head list` and retry with an existing Head name.",
        HeadError::HeadAlreadyExists(_) => {
            "Inspect it with `hydra head status <NAME>` or choose a different Head name."
        }
        HeadError::InvalidName(_) => "Run `hydra head create --help` and choose a valid Head name.",
        HeadError::InvalidConfiguration { .. }
        | HeadError::UnsupportedConfigurationVersion(_)
        | HeadError::ConcurrentConfigurationChange(_)
        | HeadError::SerializeConfiguration(_) => {
            "Review or restore the version-controlled `.hydra.json`, then retry."
        }
        HeadError::InvalidState { .. }
        | HeadError::InvalidLocalMetadata { .. }
        | HeadError::UnsupportedStateVersion(_)
        | HeadError::UnsupportedLocalMetadataVersion { .. }
        | HeadError::LocalIdentityMismatch(_)
        | HeadError::DirectoryPolicyMismatch(_)
        | HeadError::UnsafeHeadPath(_)
        | HeadError::UnsafeProjectFile(_)
        | HeadError::UnsafeHeadsDirectory(_)
        | HeadError::ConcurrentStateChange(_)
        | HeadError::StateLockExists(_)
        | HeadError::HeadOpenInconsistent { .. }
        | HeadError::HeadRemovalInconsistent { .. }
        | HeadError::HeadRemovalIncomplete { .. }
        | HeadError::HeadCloseInconsistent { .. }
        | HeadError::HeadIntegratedButRemovalFailed { .. }
        | HeadError::ConfigurationCommittedWithCleanupFailure(_)
        | HeadError::HeadCommittedWithCleanupFailure(_)
        | HeadError::RollbackFailed { .. } => {
            "Run `hydra repair` to inspect and reconcile deterministic local-state residue; preserve anything it leaves report-only."
        }
        HeadError::OpenCommandNotConfigured | HeadError::InvalidOpenCommand(_) => {
            "Review `commands.open` in `.hydra.json`, then rerun `hydra head open`."
        }
        HeadError::OpenCommandUnavailable { .. } | HeadError::OpenCommandFailed { .. } => {
            "Verify the configured open program and arguments directly, then retry; the Head was preserved."
        }
        HeadError::HeadHasUncommittedChanges(_) => {
            "Commit or stash the Head changes, or rerun with `--force` only to discard them."
        }
        HeadError::HeadHasUnintegratedCommits { .. } => {
            "Integrate with `hydra head close <NAME>`, or use forced removal only if preserving the private branch is intended."
        }
        HeadError::HeadCloseHasUncommittedChanges(_) => {
            "Commit or stash the Head changes, then rerun `hydra head close <NAME>`."
        }
        HeadError::HeadCloseRequiresParentWorktree { .. } => {
            "Change to the reported parent project worktree and rerun `hydra head close <NAME>`."
        }
        HeadError::HeadCloseRequiresTargetBranch { .. } => {
            "Check out the reported target branch in the parent worktree, then retry."
        }
        HeadError::HeadCloseAborted { .. } => {
            "Inspect the preserved Head and target, then rerun close when integration is wanted."
        }
        HeadError::HeadCloseTargetWorktreeDirty { .. } => {
            "Commit or stash changes in the reported target worktree, then retry."
        }
        HeadError::HeadCloseTargetWorktreeOperation { .. } => {
            "Finish or abort the reported Git operation in the target worktree, then retry."
        }
        HeadError::InvalidCloseCommand(_)
        | HeadError::CloseCommandUnavailable { .. }
        | HeadError::CloseCommandFailed { .. }
        | HeadError::HeadCloseCommandCompletedButRemovalFailed { .. } => {
            "Inspect `commands.close`, the preserved Head, and the target ref before retrying."
        }
        HeadError::DestinationExists(_) | HeadError::BranchAlreadyExists(_) => {
            "Run `hydra repair` to distinguish interrupted Hydra residue from an unrelated path or branch."
        }
        HeadError::InvalidRef(_) => {
            "Inspect local branches with Git and retry with an existing ref."
        }
        HeadError::TargetRequired => {
            "Pass `--target <BRANCH>` explicitly; run `hydra head create --help` for the full syntax."
        }
        HeadError::OverlayFullCopyConfirmationRequired { .. } => {
            "Rerun interactively and confirm full copy only after reviewing its cost."
        }
        HeadError::UnsafeOverlaySymlinks { .. }
        | HeadError::OverlayRules(_)
        | HeadError::UnsafeOverlayPath(_)
        | HeadError::OverlayOverwritesTracked(_)
        | HeadError::OverlayChanged(_) => {
            "Review the reported overlay and `.hydra.json`; correct the source or rule, then retry."
        }
        HeadError::UnsupportedTrackedEntry { .. } => {
            "Inspect the tracked Git entry; Hydra does not populate unsupported entry types implicitly."
        }
        HeadError::GitUnavailable(_)
        | HeadError::GitCommandFailed { .. }
        | HeadError::InvalidGitOutput(_) => {
            "Verify Git and repository health, preserve current work, then retry."
        }
        HeadError::SerializeState(_) | HeadError::Timestamp(_) => {
            "Retry with the current Hydra release; if it repeats, preserve the state and report the error."
        }
        HeadError::FileSystem { .. } => {
            "Resolve the reported filesystem problem, then run `hydra repair` before retrying a lifecycle mutation."
        }
    }
}

fn skill_next_step(error: &crate::skill::SkillError) -> &'static str {
    use crate::skill::SkillError;

    match error {
        SkillError::HomeUnavailable(_) => "Set an absolute `HOME` or `USERPROFILE`, then retry.",
        SkillError::AlreadyExists { .. } => {
            "Run `hydra skill status <PROVIDER>` and preserve unknown or modified content."
        }
        SkillError::NotInstalled { .. } => {
            "Run `hydra skill install <PROVIDER>` if installation is intended."
        }
        SkillError::Modified { .. } | SkillError::Manifest { .. } => {
            "Inspect and preserve the installed skill; back it up before any manual replacement."
        }
        SkillError::Io { .. } => {
            "Resolve the reported path, permission, or storage problem, then retry the same skill command."
        }
        SkillError::JsonOutput(_) => {
            "Fix the reported JSON output problem and rerun `hydra skill status <PROVIDER> --json`."
        }
    }
}

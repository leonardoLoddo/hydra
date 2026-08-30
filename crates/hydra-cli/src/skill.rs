use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    fmt, fs,
    io::{self, IsTerminal as _, Write as _},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

const SKILL_MD: &[u8] = include_bytes!("../../../skills/hydra/SKILL.md");
const OPENAI_YAML: &[u8] = include_bytes!("../../../skills/hydra/agents/openai.yaml");
const MANIFEST_NAME: &str = ".hydra-skill.json";

#[derive(Clone, Copy, Debug)]
pub enum Provider {
    Codex,
    Gemini,
}

impl Provider {
    fn id(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Gemini => "gemini",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Gemini => "Gemini CLI",
        }
    }

    fn destination(self, home: &Path) -> PathBuf {
        match self {
            Self::Codex => home.join(".agents/skills/hydra"),
            Self::Gemini => home.join(".gemini/skills/hydra"),
        }
    }

    fn refresh_guidance(self, updated: bool) -> &'static str {
        match (self, updated) {
            (Self::Codex, false) => {
                "Codex detects skill changes automatically; restart it only if $hydra does not appear."
            }
            (Self::Codex, true) => {
                "Codex detects skill changes automatically; restart it only if the update does not appear."
            }
            (Self::Gemini, _) => {
                "Run /skills reload in Gemini CLI if the Hydra skill does not appear or refresh."
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum Action {
    Install,
    Status,
    Update,
    Remove,
}

#[derive(Clone, Copy)]
pub struct Confirmation {
    pub yes: bool,
    pub no: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    provider: String,
    hydra_version: String,
    files: BTreeMap<String, String>,
}

enum InstalledState {
    Absent,
    Managed(Manifest),
    Modified(String),
}

#[derive(Debug)]
pub enum SkillError {
    HomeUnavailable(Provider),
    AlreadyExists {
        provider: Provider,
        path: PathBuf,
    },
    NotInstalled {
        provider: Provider,
        path: PathBuf,
    },
    Modified {
        provider: Provider,
        path: PathBuf,
        reason: String,
    },
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Manifest {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl fmt::Display for SkillError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HomeUnavailable(provider) => write!(
                formatter,
                "cannot resolve the {} skill destination because HOME is unavailable",
                provider.display_name()
            ),
            Self::AlreadyExists { provider, path } => write!(
                formatter,
                "the {} skill destination already exists and was preserved: {}",
                provider.display_name(),
                path.display()
            ),
            Self::NotInstalled { provider, path } => {
                write!(
                    formatter,
                    "the {} skill is not installed at {}",
                    provider.display_name(),
                    path.display()
                )
            }
            Self::Modified {
                provider,
                path,
                reason,
            } => write!(
                formatter,
                "the {} skill is unknown or locally modified at {}: {reason}; it was preserved",
                provider.display_name(),
                path.display()
            ),
            Self::Io {
                operation,
                path,
                source,
            } => write!(
                formatter,
                "could not {operation} {}: {source}",
                path.display()
            ),
            Self::Manifest { path, source } => {
                write!(formatter, "could not parse {}: {source}", path.display())
            }
        }
    }
}

impl Error for SkillError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Manifest { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn run(
    action: Action,
    provider: Provider,
    confirmation: Confirmation,
) -> Result<(), SkillError> {
    let destination = provider_destination(provider)?;
    match action {
        Action::Install => install(provider, &destination, confirmation),
        Action::Status => status(provider, &destination),
        Action::Update => update(provider, &destination, confirmation),
        Action::Remove => remove(provider, &destination, confirmation),
    }
}

fn provider_destination(provider: Provider) -> Result<PathBuf, SkillError> {
    let home = env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .or_else(|| env::var_os("USERPROFILE").filter(|value| !value.is_empty()))
        .ok_or(SkillError::HomeUnavailable(provider))?;
    let home = PathBuf::from(home);
    if !home.is_absolute() {
        return Err(SkillError::HomeUnavailable(provider));
    }
    Ok(provider.destination(&home))
}

fn install(
    provider: Provider,
    destination: &Path,
    confirmation: Confirmation,
) -> Result<(), SkillError> {
    if confirmation.no {
        println!("{} skill not installed.", provider.display_name());
        return Ok(());
    }
    if symlink_metadata(destination)?.is_some() {
        return Err(SkillError::AlreadyExists {
            provider,
            path: destination.to_path_buf(),
        });
    }
    if !confirmed(
        &format!(
            "Install the optional Hydra skill for {}?",
            provider.display_name()
        ),
        destination,
        confirmation,
    )? {
        println!("{} skill not installed.", provider.display_name());
        return Ok(());
    }

    publish_new(provider, destination)?;
    println!(
        "Installed {} skill at {}.",
        provider.display_name(),
        destination.display()
    );
    println!("{}", provider.refresh_guidance(false));
    Ok(())
}

fn status(provider: Provider, destination: &Path) -> Result<(), SkillError> {
    match inspect(provider, destination)? {
        InstalledState::Absent => Err(SkillError::NotInstalled {
            provider,
            path: destination.to_path_buf(),
        }),
        InstalledState::Modified(reason) => Err(SkillError::Modified {
            provider,
            path: destination.to_path_buf(),
            reason,
        }),
        InstalledState::Managed(manifest) => {
            if is_current(provider, &manifest) {
                println!(
                    "{} skill is current at {} (Hydra {}).",
                    provider.display_name(),
                    destination.display(),
                    manifest.hydra_version
                );
            } else {
                println!(
                    "{} skill is managed and unmodified at {}, but an update to Hydra {} is available.",
                    provider.display_name(),
                    destination.display(),
                    env!("CARGO_PKG_VERSION")
                );
            }
            Ok(())
        }
    }
}

fn update(
    provider: Provider,
    destination: &Path,
    confirmation: Confirmation,
) -> Result<(), SkillError> {
    if confirmation.no {
        println!("{} skill not updated.", provider.display_name());
        return Ok(());
    }
    let manifest = require_managed(provider, destination)?;
    if is_current(provider, &manifest) {
        println!(
            "{} skill is already current at {}.",
            provider.display_name(),
            destination.display()
        );
        return Ok(());
    }
    if !confirmed(
        &format!("Update the Hydra skill for {}?", provider.display_name()),
        destination,
        confirmation,
    )? {
        println!("{} skill not updated.", provider.display_name());
        return Ok(());
    }

    replace_managed(provider, destination)?;
    println!(
        "Updated {} skill at {}.",
        provider.display_name(),
        destination.display()
    );
    println!("{}", provider.refresh_guidance(true));
    Ok(())
}

fn remove(
    provider: Provider,
    destination: &Path,
    confirmation: Confirmation,
) -> Result<(), SkillError> {
    if confirmation.no {
        println!("{} skill not removed.", provider.display_name());
        return Ok(());
    }
    require_managed(provider, destination)?;
    if !confirmed(
        &format!("Remove the Hydra skill from {}?", provider.display_name()),
        destination,
        confirmation,
    )? {
        println!("{} skill not removed.", provider.display_name());
        return Ok(());
    }
    fs::remove_dir_all(destination).map_err(|source| SkillError::Io {
        operation: "remove",
        path: destination.to_path_buf(),
        source,
    })?;
    println!(
        "Removed {} skill from {}.",
        provider.display_name(),
        destination.display()
    );
    Ok(())
}

fn confirmed(
    question: &str,
    destination: &Path,
    confirmation: Confirmation,
) -> Result<bool, SkillError> {
    if confirmation.no {
        return Ok(false);
    }
    if confirmation.yes {
        return Ok(true);
    }
    if !io::stdin().is_terminal() {
        return Ok(false);
    }

    eprintln!("{question}");
    eprintln!("Destination: {}", destination.display());
    eprint!("Continue? [y/N] ");
    io::stderr().flush().map_err(|source| SkillError::Io {
        operation: "write the confirmation prompt for",
        path: destination.to_path_buf(),
        source,
    })?;
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|source| SkillError::Io {
            operation: "read confirmation for",
            path: destination.to_path_buf(),
            source,
        })?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn publish_new(provider: Provider, destination: &Path) -> Result<(), SkillError> {
    let parent = destination
        .parent()
        .ok_or(SkillError::HomeUnavailable(provider))?;
    fs::create_dir_all(parent).map_err(|source| SkillError::Io {
        operation: "create",
        path: parent.to_path_buf(),
        source,
    })?;
    let staged = stage_skill(provider, parent)?;
    if let Err(source) = fs::rename(staged.path(), destination) {
        return Err(SkillError::Io {
            operation: "publish",
            path: destination.to_path_buf(),
            source,
        });
    }
    Ok(())
}

fn replace_managed(provider: Provider, destination: &Path) -> Result<(), SkillError> {
    let parent = destination
        .parent()
        .ok_or(SkillError::HomeUnavailable(provider))?;
    let staged = stage_skill(provider, parent)?;
    let backup = tempfile::Builder::new()
        .prefix(".hydra-skill-backup-")
        .tempdir_in(parent)
        .map_err(|source| SkillError::Io {
            operation: "create a backup directory in",
            path: parent.to_path_buf(),
            source,
        })?;
    let backup_path = backup.path().join("hydra");
    fs::rename(destination, &backup_path).map_err(|source| SkillError::Io {
        operation: "stage the previous skill from",
        path: destination.to_path_buf(),
        source,
    })?;
    if let Err(source) = fs::rename(staged.path(), destination) {
        let _ = fs::rename(&backup_path, destination);
        return Err(SkillError::Io {
            operation: "publish the updated skill at",
            path: destination.to_path_buf(),
            source,
        });
    }
    if let Err(source) = backup.close() {
        eprintln!("warning: updated the skill but could not remove its temporary backup: {source}");
    }
    Ok(())
}

fn stage_skill(provider: Provider, parent: &Path) -> Result<tempfile::TempDir, SkillError> {
    let staged = tempfile::Builder::new()
        .prefix(".hydra-skill-install-")
        .tempdir_in(parent)
        .map_err(|source| SkillError::Io {
            operation: "create a staging directory in",
            path: parent.to_path_buf(),
            source,
        })?;
    let agents = staged.path().join("agents");
    fs::create_dir(&agents).map_err(|source| SkillError::Io {
        operation: "create",
        path: agents.clone(),
        source,
    })?;
    write_file(&staged.path().join("SKILL.md"), SKILL_MD)?;
    write_file(&agents.join("openai.yaml"), OPENAI_YAML)?;
    let mut manifest_bytes =
        serde_json::to_vec_pretty(&current_manifest(provider)).map_err(|source| {
            SkillError::Manifest {
                path: staged.path().join(MANIFEST_NAME),
                source,
            }
        })?;
    manifest_bytes.push(b'\n');
    write_file(&staged.path().join(MANIFEST_NAME), &manifest_bytes)?;
    Ok(staged)
}

fn write_file(path: &Path, contents: &[u8]) -> Result<(), SkillError> {
    fs::write(path, contents).map_err(|source| SkillError::Io {
        operation: "write",
        path: path.to_path_buf(),
        source,
    })
}

fn require_managed(provider: Provider, destination: &Path) -> Result<Manifest, SkillError> {
    match inspect(provider, destination)? {
        InstalledState::Absent => Err(SkillError::NotInstalled {
            provider,
            path: destination.to_path_buf(),
        }),
        InstalledState::Modified(reason) => Err(SkillError::Modified {
            provider,
            path: destination.to_path_buf(),
            reason,
        }),
        InstalledState::Managed(manifest) => Ok(manifest),
    }
}

fn inspect(provider: Provider, destination: &Path) -> Result<InstalledState, SkillError> {
    let Some(metadata) = symlink_metadata(destination)? else {
        return Ok(InstalledState::Absent);
    };
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Ok(InstalledState::Modified(
            "the destination is not a regular directory".to_owned(),
        ));
    }

    let expected_root = BTreeSet::from([
        MANIFEST_NAME.to_owned(),
        "SKILL.md".to_owned(),
        "agents".to_owned(),
    ]);
    if directory_names(destination)? != expected_root {
        return Ok(InstalledState::Modified(
            "the installed directory contains missing or extra entries".to_owned(),
        ));
    }
    let agents = destination.join("agents");
    if directory_names(&agents)? != BTreeSet::from(["openai.yaml".to_owned()]) {
        return Ok(InstalledState::Modified(
            "the agents directory contains missing or extra entries".to_owned(),
        ));
    }
    for path in [
        destination.join("SKILL.md"),
        destination.join("agents/openai.yaml"),
        destination.join(MANIFEST_NAME),
    ] {
        let Some(metadata) = symlink_metadata(&path)? else {
            return Ok(InstalledState::Modified(format!(
                "{} is missing",
                path.display()
            )));
        };
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Ok(InstalledState::Modified(format!(
                "{} is not a regular file",
                path.display()
            )));
        }
    }

    let manifest_path = destination.join(MANIFEST_NAME);
    let manifest_bytes = read_file(&manifest_path)?;
    let manifest: Manifest = match serde_json::from_slice(&manifest_bytes) {
        Ok(manifest) => manifest,
        Err(source) => {
            return Err(SkillError::Manifest {
                path: manifest_path,
                source,
            });
        }
    };
    if manifest.schema_version != 1 || manifest.provider != provider.id() {
        return Ok(InstalledState::Modified(format!(
            "the provenance manifest is not owned by the supported {} adapter",
            provider.display_name()
        )));
    }
    let expected_files = BTreeSet::from(["SKILL.md", "agents/openai.yaml"]);
    if manifest
        .files
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected_files
    {
        return Ok(InstalledState::Modified(
            "the provenance manifest does not describe the exact skill".to_owned(),
        ));
    }
    for (relative, expected_digest) in &manifest.files {
        let actual = sha256_hex(&read_file(&destination.join(relative))?);
        if &actual != expected_digest {
            return Ok(InstalledState::Modified(format!(
                "{relative} does not match its installed checksum"
            )));
        }
    }
    Ok(InstalledState::Managed(manifest))
}

fn directory_names(path: &Path) -> Result<BTreeSet<String>, SkillError> {
    let entries = fs::read_dir(path).map_err(|source| SkillError::Io {
        operation: "read",
        path: path.to_path_buf(),
        source,
    })?;
    let mut names = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|source| SkillError::Io {
            operation: "read an entry in",
            path: path.to_path_buf(),
            source,
        })?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            return Ok(BTreeSet::from(["<non-UTF-8-entry>".to_owned()]));
        };
        names.insert(name);
    }
    Ok(names)
}

fn symlink_metadata(path: &Path) -> Result<Option<fs::Metadata>, SkillError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(SkillError::Io {
            operation: "inspect",
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn read_file(path: &Path) -> Result<Vec<u8>, SkillError> {
    fs::read(path).map_err(|source| SkillError::Io {
        operation: "read",
        path: path.to_path_buf(),
        source,
    })
}

fn current_manifest(provider: Provider) -> Manifest {
    Manifest {
        schema_version: 1,
        provider: provider.id().to_owned(),
        hydra_version: env!("CARGO_PKG_VERSION").to_owned(),
        files: BTreeMap::from([
            ("SKILL.md".to_owned(), sha256_hex(SKILL_MD)),
            ("agents/openai.yaml".to_owned(), sha256_hex(OPENAI_YAML)),
        ]),
    }
}

fn is_current(provider: Provider, manifest: &Manifest) -> bool {
    manifest.hydra_version == env!("CARGO_PKG_VERSION")
        && manifest.files == current_manifest(provider).files
}

fn sha256_hex(contents: &[u8]) -> String {
    use fmt::Write as _;

    Sha256::digest(contents)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        })
}

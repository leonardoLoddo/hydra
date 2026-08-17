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
    HomeUnavailable,
    AlreadyExists(PathBuf),
    NotInstalled(PathBuf),
    Modified {
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
            Self::HomeUnavailable => write!(
                formatter,
                "cannot resolve the Codex skill destination because HOME is unavailable"
            ),
            Self::AlreadyExists(path) => write!(
                formatter,
                "the Codex skill destination already exists and was preserved: {}",
                path.display()
            ),
            Self::NotInstalled(path) => {
                write!(
                    formatter,
                    "the Codex skill is not installed at {}",
                    path.display()
                )
            }
            Self::Modified { path, reason } => write!(
                formatter,
                "the Codex skill is unknown or locally modified at {}: {reason}; it was preserved",
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

pub fn run(action: Action, confirmation: Confirmation) -> Result<(), SkillError> {
    let destination = codex_destination()?;
    match action {
        Action::Install => install(&destination, confirmation),
        Action::Status => status(&destination),
        Action::Update => update(&destination, confirmation),
        Action::Remove => remove(&destination, confirmation),
    }
}

fn codex_destination() -> Result<PathBuf, SkillError> {
    let home = env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .or_else(|| env::var_os("USERPROFILE").filter(|value| !value.is_empty()))
        .ok_or(SkillError::HomeUnavailable)?;
    let home = PathBuf::from(home);
    if !home.is_absolute() {
        return Err(SkillError::HomeUnavailable);
    }
    Ok(home.join(".agents/skills/hydra"))
}

fn install(destination: &Path, confirmation: Confirmation) -> Result<(), SkillError> {
    if confirmation.no {
        println!("Codex skill not installed.");
        return Ok(());
    }
    if symlink_metadata(destination)?.is_some() {
        return Err(SkillError::AlreadyExists(destination.to_path_buf()));
    }
    if !confirmed(
        "Install the optional Hydra skill for Codex?",
        destination,
        confirmation,
    )? {
        println!("Codex skill not installed.");
        return Ok(());
    }

    publish_new(destination)?;
    println!("Installed Codex skill at {}.", destination.display());
    println!(
        "Codex detects skill changes automatically; restart it only if $hydra does not appear."
    );
    Ok(())
}

fn status(destination: &Path) -> Result<(), SkillError> {
    match inspect(destination)? {
        InstalledState::Absent => Err(SkillError::NotInstalled(destination.to_path_buf())),
        InstalledState::Modified(reason) => Err(SkillError::Modified {
            path: destination.to_path_buf(),
            reason,
        }),
        InstalledState::Managed(manifest) => {
            if is_current(&manifest) {
                println!(
                    "Codex skill is current at {} (Hydra {}).",
                    destination.display(),
                    manifest.hydra_version
                );
            } else {
                println!(
                    "Codex skill is managed and unmodified at {}, but an update to Hydra {} is available.",
                    destination.display(),
                    env!("CARGO_PKG_VERSION")
                );
            }
            Ok(())
        }
    }
}

fn update(destination: &Path, confirmation: Confirmation) -> Result<(), SkillError> {
    if confirmation.no {
        println!("Codex skill not updated.");
        return Ok(());
    }
    let manifest = require_managed(destination)?;
    if is_current(&manifest) {
        println!(
            "Codex skill is already current at {}.",
            destination.display()
        );
        return Ok(());
    }
    if !confirmed(
        "Update the Hydra skill for Codex?",
        destination,
        confirmation,
    )? {
        println!("Codex skill not updated.");
        return Ok(());
    }

    replace_managed(destination)?;
    println!("Updated Codex skill at {}.", destination.display());
    println!(
        "Codex detects skill changes automatically; restart it only if the update does not appear."
    );
    Ok(())
}

fn remove(destination: &Path, confirmation: Confirmation) -> Result<(), SkillError> {
    if confirmation.no {
        println!("Codex skill not removed.");
        return Ok(());
    }
    require_managed(destination)?;
    if !confirmed(
        "Remove the Hydra skill from Codex?",
        destination,
        confirmation,
    )? {
        println!("Codex skill not removed.");
        return Ok(());
    }
    fs::remove_dir_all(destination).map_err(|source| SkillError::Io {
        operation: "remove",
        path: destination.to_path_buf(),
        source,
    })?;
    println!("Removed Codex skill from {}.", destination.display());
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

fn publish_new(destination: &Path) -> Result<(), SkillError> {
    let parent = destination.parent().ok_or(SkillError::HomeUnavailable)?;
    fs::create_dir_all(parent).map_err(|source| SkillError::Io {
        operation: "create",
        path: parent.to_path_buf(),
        source,
    })?;
    let staged = stage_skill(parent)?;
    if let Err(source) = fs::rename(staged.path(), destination) {
        return Err(SkillError::Io {
            operation: "publish",
            path: destination.to_path_buf(),
            source,
        });
    }
    Ok(())
}

fn replace_managed(destination: &Path) -> Result<(), SkillError> {
    let parent = destination.parent().ok_or(SkillError::HomeUnavailable)?;
    let staged = stage_skill(parent)?;
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

fn stage_skill(parent: &Path) -> Result<tempfile::TempDir, SkillError> {
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
        serde_json::to_vec_pretty(&current_manifest()).map_err(|source| SkillError::Manifest {
            path: staged.path().join(MANIFEST_NAME),
            source,
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

fn require_managed(destination: &Path) -> Result<Manifest, SkillError> {
    match inspect(destination)? {
        InstalledState::Absent => Err(SkillError::NotInstalled(destination.to_path_buf())),
        InstalledState::Modified(reason) => Err(SkillError::Modified {
            path: destination.to_path_buf(),
            reason,
        }),
        InstalledState::Managed(manifest) => Ok(manifest),
    }
}

fn inspect(destination: &Path) -> Result<InstalledState, SkillError> {
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
    if manifest.schema_version != 1 || manifest.provider != "codex" {
        return Ok(InstalledState::Modified(
            "the provenance manifest is not owned by the supported Codex adapter".to_owned(),
        ));
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

fn current_manifest() -> Manifest {
    Manifest {
        schema_version: 1,
        provider: "codex".to_owned(),
        hydra_version: env!("CARGO_PKG_VERSION").to_owned(),
        files: BTreeMap::from([
            ("SKILL.md".to_owned(), sha256_hex(SKILL_MD)),
            ("agents/openai.yaml".to_owned(), sha256_hex(OPENAI_YAML)),
        ]),
    }
}

fn is_current(manifest: &Manifest) -> bool {
    manifest.hydra_version == env!("CARGO_PKG_VERSION")
        && manifest.files == current_manifest().files
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

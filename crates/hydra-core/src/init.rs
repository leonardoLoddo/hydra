use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;
use uuid::Uuid;

const CONFIG_FILE_NAME: &str = ".hydra.json";
const STATE_DIRECTORY_NAME: &str = "hydra";
const STATE_FILE_NAME: &str = "heads.json";

#[derive(Debug)]
pub struct InitializedProject {
    pub repository_root: PathBuf,
    pub heads_directory: PathBuf,
}

#[derive(Debug)]
pub enum InitError {
    GitUnavailable(std::io::Error),
    NotGitRepository(PathBuf),
    InvalidGitOutput(&'static str),
    UnsupportedRepositoryPath(PathBuf),
    AlreadyInitialized(PathBuf),
    HeadsDirectoryExists(PathBuf),
    LocalStateExists(PathBuf),
    SerializeConfiguration(serde_json::Error),
    FileSystem {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for InitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitUnavailable(error) => write!(formatter, "could not run Git: {error}"),
            Self::NotGitRepository(path) => {
                write!(formatter, "{} is not a Git repository", path.display())
            }
            Self::InvalidGitOutput(field) => {
                write!(formatter, "Git returned an invalid {field}")
            }
            Self::UnsupportedRepositoryPath(path) => write!(
                formatter,
                "cannot derive a sibling Heads directory for {}",
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
        }
    }
}

impl Error for InitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::GitUnavailable(error) => Some(error),
            Self::SerializeConfiguration(error) => Some(error),
            Self::FileSystem { source, .. } => Some(source),
            Self::NotGitRepository(_)
            | Self::InvalidGitOutput(_)
            | Self::UnsupportedRepositoryPath(_)
            | Self::AlreadyInitialized(_)
            | Self::HeadsDirectoryExists(_)
            | Self::LocalStateExists(_) => None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectConfiguration {
    version: u32,
    project_id: String,
    heads_directory: String,
    branch_prefix: String,
    storage: StorageConfiguration,
    overlay: OverlayConfiguration,
}

#[derive(Serialize)]
struct StorageConfiguration {
    mode: String,
}

#[derive(Serialize)]
struct OverlayConfiguration {
    copy: Vec<String>,
}

#[derive(Serialize)]
struct LocalState {
    version: u32,
    heads: BTreeMap<String, serde_json::Value>,
}

struct Repository {
    root: PathBuf,
    git_common_directory: PathBuf,
}

/// Initializes Hydra metadata for the Git repository containing `path`.
///
/// # Errors
///
/// Returns [`InitError`] when Git cannot resolve the repository, a destination
/// already exists, configuration cannot be serialized, or a filesystem
/// operation fails. Validation errors occur before Hydra creates any artifact.
pub fn initialize(path: &Path) -> Result<InitializedProject, InitError> {
    let repository = discover_repository(path)?;
    let repository_name = repository
        .root
        .file_name()
        .ok_or_else(|| InitError::UnsupportedRepositoryPath(repository.root.clone()))?;
    let repository_parent = repository
        .root
        .parent()
        .ok_or_else(|| InitError::UnsupportedRepositoryPath(repository.root.clone()))?;

    let heads_name = format!("{}.heads", repository_name.to_string_lossy());
    let heads_directory = repository_parent.join(&heads_name);
    let configuration_path = repository.root.join(CONFIG_FILE_NAME);
    let state_directory = repository.git_common_directory.join(STATE_DIRECTORY_NAME);
    let state_path = state_directory.join(STATE_FILE_NAME);

    validate_destinations(&configuration_path, &heads_directory, &state_path)?;

    let configuration = ProjectConfiguration {
        version: 1,
        project_id: generate_project_id(repository_name),
        heads_directory: Path::new("..")
            .join(&heads_name)
            .to_string_lossy()
            .into_owned(),
        branch_prefix: "hydra/".to_owned(),
        storage: StorageConfiguration {
            mode: "auto".to_owned(),
        },
        overlay: OverlayConfiguration {
            copy: vec!["... .gitignore".to_owned()],
        },
    };
    let state = LocalState {
        version: 1,
        heads: BTreeMap::new(),
    };

    let configuration_bytes = serialize_json(&configuration)?;
    let state_bytes = serialize_json(&state)?;
    let files = InitialFiles {
        heads_directory: &heads_directory,
        state_directory: &state_directory,
        state_path: &state_path,
        configuration_path: &configuration_path,
    };
    create_initial_files(&files, &state_bytes, &configuration_bytes)?;

    Ok(InitializedProject {
        repository_root: repository.root,
        heads_directory,
    })
}

fn discover_repository(path: &Path) -> Result<Repository, InitError> {
    let root = git_path(path, "--show-toplevel", "repository root")?;
    let git_common_directory = git_path(&root, "--git-common-dir", "Git common directory")?;

    Ok(Repository {
        root,
        git_common_directory,
    })
}

fn git_path(path: &Path, argument: &str, field: &'static str) -> Result<PathBuf, InitError> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(path)
        .args(["rev-parse", "--path-format=absolute", argument])
        .output()
        .map_err(InitError::GitUnavailable)?;

    if !output.status.success() {
        return Err(InitError::NotGitRepository(path.to_path_buf()));
    }

    let value = std::str::from_utf8(&output.stdout)
        .map_err(|_| InitError::InvalidGitOutput(field))?
        .trim_end();
    if value.is_empty() {
        return Err(InitError::InvalidGitOutput(field));
    }

    Ok(PathBuf::from(value))
}

fn validate_destinations(
    configuration_path: &Path,
    heads_directory: &Path,
    state_path: &Path,
) -> Result<(), InitError> {
    if configuration_path.exists() {
        return Err(InitError::AlreadyInitialized(
            configuration_path.to_path_buf(),
        ));
    }
    if heads_directory.exists() {
        return Err(InitError::HeadsDirectoryExists(
            heads_directory.to_path_buf(),
        ));
    }
    if state_path.exists() {
        return Err(InitError::LocalStateExists(state_path.to_path_buf()));
    }
    Ok(())
}

fn generate_project_id(repository_name: &std::ffi::OsStr) -> String {
    let mut slug = repository_name
        .to_string_lossy()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "project" } else { slug };
    let suffix = Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>();
    format!("{slug}-{suffix}")
}

fn serialize_json<T: Serialize>(value: &T) -> Result<Vec<u8>, InitError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(InitError::SerializeConfiguration)?;
    bytes.push(b'\n');
    Ok(bytes)
}

struct InitialFiles<'a> {
    heads_directory: &'a Path,
    state_directory: &'a Path,
    state_path: &'a Path,
    configuration_path: &'a Path,
}

fn create_initial_files(
    files: &InitialFiles<'_>,
    state_bytes: &[u8],
    configuration_bytes: &[u8],
) -> Result<(), InitError> {
    create_directory(files.heads_directory, "create Heads directory")?;

    let state_directory_preexisted = files.state_directory.exists();
    if let Err(error) = create_directory_if_missing(files.state_directory, "create state directory")
    {
        remove_directory_if_empty(files.heads_directory);
        return Err(error);
    }

    if let Err(error) = write_atomic(files.state_path, state_bytes) {
        remove_directory_if_empty(files.heads_directory);
        if !state_directory_preexisted {
            remove_directory_if_empty(files.state_directory);
        }
        return Err(error);
    }

    if let Err(error) = write_atomic(files.configuration_path, configuration_bytes) {
        remove_file_if_present(files.state_path);
        remove_directory_if_empty(files.heads_directory);
        if !state_directory_preexisted {
            remove_directory_if_empty(files.state_directory);
        }
        return Err(error);
    }

    Ok(())
}

fn create_directory(path: &Path, action: &'static str) -> Result<(), InitError> {
    fs::create_dir(path).map_err(|source| InitError::FileSystem {
        action,
        path: path.to_path_buf(),
        source,
    })
}

fn create_directory_if_missing(path: &Path, action: &'static str) -> Result<(), InitError> {
    fs::create_dir_all(path).map_err(|source| InitError::FileSystem {
        action,
        path: path.to_path_buf(),
        source,
    })
}

fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), InitError> {
    let file_name = path
        .file_name()
        .ok_or_else(|| InitError::UnsupportedRepositoryPath(path.to_path_buf()))?;
    let temporary_path = path.with_file_name(format!(
        ".{}.tmp-{}",
        file_name.to_string_lossy(),
        Uuid::new_v4().simple()
    ));

    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|source| InitError::FileSystem {
                action: "create temporary file",
                path: temporary_path.clone(),
                source,
            })?;
        file.write_all(contents)
            .and_then(|()| file.sync_all())
            .map_err(|source| InitError::FileSystem {
                action: "write temporary file",
                path: temporary_path.clone(),
                source,
            })?;
        fs::rename(&temporary_path, path).map_err(|source| InitError::FileSystem {
            action: "publish file atomically",
            path: path.to_path_buf(),
            source,
        })
    })();

    if result.is_err() {
        remove_file_if_present(&temporary_path);
    }
    result
}

fn remove_file_if_present(path: &Path) {
    if let Err(error) = fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        // Best-effort rollback; the original error remains the actionable failure.
    }
}

fn remove_directory_if_empty(path: &Path) {
    if let Err(error) = fs::remove_dir(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        // Best-effort rollback; non-empty directories are deliberately preserved.
    }
}

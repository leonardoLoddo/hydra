mod artifacts;
mod configuration;
mod error;
mod git;
mod persistence;
mod storage;

use std::{
    fs,
    path::{Path, PathBuf},
};

use configuration::{serialize_initial_configuration, serialize_initial_state};
use git::{discover_repository, repository_name_as_str};
use persistence::{InitialFiles, create_initial_files};

pub use error::{CleanupFailure, InitError};
pub use storage::StorageBackend;

const CONFIG_FILE_NAME: &str = ".hydra.json";
const STATE_DIRECTORY_NAME: &str = "hydra";
const STATE_FILE_NAME: &str = "heads.json";

#[derive(Debug)]
pub struct InitializedProject {
    pub repository_root: PathBuf,
    pub heads_directory: PathBuf,
    pub storage_backend: StorageBackend,
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
    let repository_name = repository_name_as_str(repository_name)?;
    let repository_parent = repository
        .root
        .parent()
        .ok_or_else(|| InitError::UnsupportedRepositoryPath(repository.root.clone()))?;

    let heads_name = format!("{repository_name}.heads");
    let heads_directory = repository_parent.join(&heads_name);
    let configuration_path = repository.root.join(CONFIG_FILE_NAME);
    let state_directory = repository.git_common_directory.join(STATE_DIRECTORY_NAME);
    let state_path = state_directory.join(STATE_FILE_NAME);

    validate_destinations(
        &configuration_path,
        &heads_directory,
        &state_directory,
        &state_path,
    )?;

    let configuration_bytes = serialize_initial_configuration(repository_name, &heads_name)?;
    let state_bytes = serialize_initial_state()?;
    let files = InitialFiles {
        heads_directory: &heads_directory,
        state_directory: &state_directory,
        state_path: &state_path,
        configuration_path: &configuration_path,
    };
    let storage_backend = create_initial_files(&files, &state_bytes, &configuration_bytes)?;

    Ok(InitializedProject {
        repository_root: repository.root,
        heads_directory,
        storage_backend,
    })
}

fn validate_destinations(
    configuration_path: &Path,
    heads_directory: &Path,
    state_directory: &Path,
    state_path: &Path,
) -> Result<(), InitError> {
    if path_entry_exists(configuration_path, "inspect project configuration")? {
        return Err(InitError::AlreadyInitialized(
            configuration_path.to_path_buf(),
        ));
    }
    if path_entry_exists(heads_directory, "inspect Heads directory")? {
        return Err(InitError::HeadsDirectoryExists(
            heads_directory.to_path_buf(),
        ));
    }
    match fs::symlink_metadata(state_directory) {
        Ok(metadata) if !metadata.is_dir() => {
            return Err(InitError::UnsafeStateDirectory(
                state_directory.to_path_buf(),
            ));
        }
        Ok(_) if path_entry_exists(state_path, "inspect local state")? => {
            return Err(InitError::LocalStateExists(state_path.to_path_buf()));
        }
        Ok(_) => {
            return Err(InitError::StateDirectoryExists(
                state_directory.to_path_buf(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(InitError::FileSystem {
                action: "inspect state directory",
                path: state_directory.to_path_buf(),
                source,
            });
        }
    }
    Ok(())
}

fn path_entry_exists(path: &Path, action: &'static str) -> Result<bool, InitError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(InitError::FileSystem {
            action,
            path: path.to_path_buf(),
            source,
        }),
    }
}

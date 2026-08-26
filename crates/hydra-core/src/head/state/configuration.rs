use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
};

#[cfg(unix)]
use std::fs::File;

use serde::{Deserialize, Serialize, de::Error as _};
use uuid::Uuid;

use super::super::HeadError;

const CONFIG_FILE_NAME: &str = ".hydra.json";
const SUPPORTED_CONFIGURATION_VERSION: u32 = 2;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ProjectConfiguration {
    version: u32,
    project_id: String,
    heads_directory: HeadsDirectoryPolicy,
    branch_prefix: String,
    storage: StorageConfiguration,
    overlay: OverlayConfiguration,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    commands: Option<CommandConfiguration>,
    #[serde(skip)]
    path: PathBuf,
    #[serde(skip)]
    original_bytes: Vec<u8>,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "strategy", rename_all = "camelCase", deny_unknown_fields)]
enum HeadsDirectoryPolicy {
    Sibling {
        #[serde(deserialize_with = "deserialize_suffix")]
        suffix: String,
    },
    Relative {
        base: RelativeBase,
        #[serde(deserialize_with = "deserialize_relative_path")]
        path: String,
    },
    Local,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum RelativeBase {
    RepositoryParent,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StorageConfiguration {
    mode: StorageMode,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
enum StorageMode {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "copy")]
    Copy,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OverlayConfiguration {
    copy: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CommandConfiguration {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    open: Option<OpenCommandConfiguration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    close: Option<CloseCommandConfiguration>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::head) struct OpenCommandConfiguration {
    program: String,
    args: Vec<String>,
}

impl OpenCommandConfiguration {
    pub(in crate::head) fn program(&self) -> &str {
        &self.program
    }

    pub(in crate::head) fn args(&self) -> &[String] {
        &self.args
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "strategy", rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::head) enum CloseCommandConfiguration {
    Command {
        program: String,
        args: Vec<String>,
        #[serde(rename = "removeOnSuccess")]
        remove_on_success: bool,
    },
}

impl CloseCommandConfiguration {
    pub(in crate::head) fn command(&self) -> (&str, &[String], bool) {
        match self {
            Self::Command {
                program,
                args,
                remove_on_success,
            } => (program, args, *remove_on_success),
        }
    }
}

impl ProjectConfiguration {
    pub(super) fn load(repository_root: &Path) -> Result<Self, HeadError> {
        let path = repository_root.join(CONFIG_FILE_NAME);
        validate_regular_file(&path, true)?;
        let bytes = fs::read(&path).map_err(|source| HeadError::FileSystem {
            action: "read project configuration",
            path: path.clone(),
            source,
        })?;
        let mut configuration: Self =
            serde_json::from_slice(&bytes).map_err(|source| HeadError::InvalidConfiguration {
                path: path.clone(),
                source,
            })?;
        if configuration.version != SUPPORTED_CONFIGURATION_VERSION {
            return Err(HeadError::UnsupportedConfigurationVersion(
                configuration.version,
            ));
        }
        configuration.path = path;
        configuration.original_bytes = bytes;
        Ok(configuration)
    }

    pub(super) fn project_id(&self) -> &str {
        &self.project_id
    }

    pub(super) fn branch_prefix(&self) -> &str {
        &self.branch_prefix
    }

    pub(super) fn overlay_rules(&self) -> &[String] {
        &self.overlay.copy
    }

    pub(super) fn force_full_copy(&self) -> bool {
        matches!(self.storage.mode, StorageMode::Copy)
    }

    pub(super) fn open_command(&self) -> Option<&OpenCommandConfiguration> {
        self.commands
            .as_ref()
            .and_then(|commands| commands.open.as_ref())
    }

    pub(super) fn close_command(&self) -> Option<&CloseCommandConfiguration> {
        self.commands
            .as_ref()
            .and_then(|commands| commands.close.as_ref())
    }

    pub(super) fn exclude_unsafe_overlay_symlinks(
        &mut self,
        paths: &[PathBuf],
    ) -> Result<(), HeadError> {
        let rules = paths
            .iter()
            .map(|path| literal_overlay_exclusion(path))
            .collect::<Result<Vec<_>, _>>()?;
        self.overlay.copy.extend(rules);

        let mut replacement =
            serde_json::to_vec_pretty(self).map_err(HeadError::SerializeConfiguration)?;
        replacement.push(b'\n');
        replace_configuration_atomically(&self.path, &self.original_bytes, &replacement)?;
        self.original_bytes = replacement;
        Ok(())
    }

    pub(super) fn resolve_heads_directory(
        &self,
        project_root: &Path,
        located_heads: &Path,
    ) -> Result<PathBuf, HeadError> {
        match &self.heads_directory {
            HeadsDirectoryPolicy::Sibling { suffix } => {
                let parent = project_root
                    .parent()
                    .ok_or_else(|| HeadError::UnsafeHeadsDirectory(project_root.to_path_buf()))?;
                let name = project_root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| HeadError::UnsafeHeadsDirectory(project_root.to_path_buf()))?;
                Ok(parent.join(format!("{name}{suffix}")))
            }
            HeadsDirectoryPolicy::Relative {
                base: RelativeBase::RepositoryParent,
                path,
            } => {
                let parent = project_root
                    .parent()
                    .ok_or_else(|| HeadError::UnsafeHeadsDirectory(project_root.to_path_buf()))?;
                Ok(parent.join(path))
            }
            HeadsDirectoryPolicy::Local => Ok(located_heads.to_path_buf()),
        }
    }
}

fn literal_overlay_exclusion(path: &Path) -> Result<String, HeadError> {
    validate_overlay_exclusion_path(path)?;
    let mut rule = String::from("!/");
    let mut first_component = true;
    for component in path.components() {
        let Component::Normal(component) = component else {
            return Err(HeadError::UnsafeOverlayPath(path.to_path_buf()));
        };
        let component = component
            .to_str()
            .ok_or_else(|| HeadError::UnsafeOverlayPath(path.to_path_buf()))?;
        if !first_component {
            rule.push('/');
        }
        first_component = false;
        for character in component.chars() {
            if matches!(character, '\\' | '*' | '?' | '[' | ']' | '!' | '#' | ' ') {
                rule.push('\\');
            }
            rule.push(character);
        }
    }
    Ok(rule)
}

fn validate_overlay_exclusion_path(path: &Path) -> Result<(), HeadError> {
    if path.as_os_str().is_empty()
        || path.components().any(|component| {
            !matches!(component, Component::Normal(value) if value
                .to_str()
                .is_some_and(|value| !value.chars().any(char::is_control)))
        })
    {
        Err(HeadError::UnsafeOverlayPath(path.to_path_buf()))
    } else {
        Ok(())
    }
}

fn replace_configuration_atomically(
    path: &Path,
    expected: &[u8],
    replacement: &[u8],
) -> Result<(), HeadError> {
    let current = fs::read(path).map_err(|source| HeadError::FileSystem {
        action: "re-read Hydra configuration",
        path: path.to_path_buf(),
        source,
    })?;
    if current != expected {
        return Err(HeadError::ConcurrentConfigurationChange(path.to_path_buf()));
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| HeadError::UnsafeProjectFile(path.to_path_buf()))?;
    let temporary = path.with_file_name(format!(
        ".{}.tmp-{}",
        file_name.to_string_lossy(),
        Uuid::new_v4().simple()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|source| HeadError::FileSystem {
            action: "create temporary Hydra configuration",
            path: temporary.clone(),
            source,
        })?;
    if let Err(source) = file.write_all(replacement).and_then(|()| file.sync_all()) {
        drop(file);
        return Err(remove_temporary_configuration_after_error(
            &temporary,
            HeadError::FileSystem {
                action: "write temporary Hydra configuration",
                path: temporary.clone(),
                source,
            },
        ));
    }
    drop(file);

    if let Err(source) = fs::rename(&temporary, path) {
        return Err(remove_temporary_configuration_after_error(
            &temporary,
            HeadError::FileSystem {
                action: "publish Hydra configuration",
                path: path.to_path_buf(),
                source,
            },
        ));
    }
    sync_configuration_parent(path)
        .map_err(|error| HeadError::ConfigurationCommittedWithCleanupFailure(Box::new(error)))
}

fn remove_temporary_configuration_after_error(path: &Path, original: HeadError) -> HeadError {
    match fs::remove_file(path) {
        Ok(()) => original,
        Err(cleanup) => HeadError::RollbackFailed {
            original: Box::new(original),
            failures: vec![
                HeadError::FileSystem {
                    action: "remove temporary Hydra configuration",
                    path: path.to_path_buf(),
                    source: cleanup,
                }
                .to_string(),
            ],
        },
    }
}

#[cfg(unix)]
fn sync_configuration_parent(path: &Path) -> Result<(), HeadError> {
    let parent = path
        .parent()
        .ok_or_else(|| HeadError::UnsafeProjectFile(path.to_path_buf()))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| HeadError::FileSystem {
            action: "synchronize Hydra configuration directory",
            path: parent.to_path_buf(),
            source,
        })
}

#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)]
fn sync_configuration_parent(_path: &Path) -> Result<(), HeadError> {
    Ok(())
}

fn deserialize_suffix<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let suffix = String::deserialize(deserializer)?;
    if suffix.is_empty()
        || suffix
            .chars()
            .any(|character| character.is_control() || matches!(character, '/' | '\\'))
    {
        Err(D::Error::custom(
            "suffix must be a non-empty filename fragment without separators or control characters",
        ))
    } else {
        Ok(suffix)
    }
}

fn deserialize_relative_path<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let path = String::deserialize(deserializer)?;
    let valid = !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|component| !component.is_empty() && !matches!(component, "." | ".."));
    if valid {
        Ok(path)
    } else {
        Err(D::Error::custom(
            "relative Heads path must contain only normal portable components",
        ))
    }
}

fn validate_regular_file(path: &Path, configuration: bool) -> Result<(), HeadError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => Err(HeadError::UnsafeProjectFile(path.to_path_buf())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && configuration => {
            Err(HeadError::ProjectNotInitialized(path.to_path_buf()))
        }
        Err(source) => Err(HeadError::FileSystem {
            action: "inspect Hydra project file",
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{literal_overlay_exclusion, replace_configuration_atomically};
    use crate::head::HeadError;

    #[test]
    fn literal_exclusions_escape_gitignore_metacharacters_and_spaces() {
        let rule = literal_overlay_exclusion(Path::new("deps/[local] *?#!/bin"))
            .expect("portable path should become an exclusion");

        assert_eq!(rule, "!/deps/\\[local\\]\\ \\*\\?\\#\\!/bin");
    }

    #[test]
    fn atomic_configuration_replacement_preserves_a_concurrent_edit() {
        let temporary = tempfile::tempdir().expect("temporary directory should be created");
        let path = temporary.path().join(".hydra.json");
        let original = b"{\"version\":2}\n";
        let concurrent = b"{\"version\":2,\"concurrent\":true}\n";
        fs::write(&path, original).expect("original configuration should be written");
        fs::write(&path, concurrent).expect("concurrent edit should be written");

        let error = replace_configuration_atomically(
            &path,
            original,
            b"{\"version\":2,\"replacement\":true}\n",
        )
        .expect_err("a concurrent edit must not be overwritten");

        assert!(matches!(
            error,
            HeadError::ConcurrentConfigurationChange(changed) if changed == path
        ));
        assert_eq!(
            fs::read(&path).expect("configuration should remain readable"),
            concurrent
        );
    }
}

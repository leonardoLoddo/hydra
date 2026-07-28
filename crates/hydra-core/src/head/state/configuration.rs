use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, de::Error as _};

use super::super::HeadError;

const CONFIG_FILE_NAME: &str = ".hydra.json";
const SUPPORTED_CONFIGURATION_VERSION: u32 = 2;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ProjectConfiguration {
    version: u32,
    project_id: String,
    heads_directory: HeadsDirectoryPolicy,
    branch_prefix: String,
    #[serde(rename = "storage")]
    _storage: StorageConfiguration,
    overlay: OverlayConfiguration,
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum RelativeBase {
    RepositoryParent,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageConfiguration {
    #[serde(rename = "mode")]
    _mode: StorageMode,
}

#[derive(Deserialize)]
enum StorageMode {
    #[serde(rename = "auto")]
    Auto,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OverlayConfiguration {
    copy: Vec<String>,
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
        let configuration: Self = serde_json::from_slice(&bytes)
            .map_err(|source| HeadError::InvalidConfiguration { path, source })?;
        if configuration.version != SUPPORTED_CONFIGURATION_VERSION {
            return Err(HeadError::UnsupportedConfigurationVersion(
                configuration.version,
            ));
        }
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

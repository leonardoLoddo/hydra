use std::{collections::BTreeMap, path::Path};

use serde::Serialize;
use uuid::Uuid;

use super::InitError;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectConfiguration {
    version: u32,
    project_id: String,
    heads_directory: HeadsDirectoryPolicy,
    branch_prefix: String,
    storage: StorageConfiguration,
    overlay: OverlayConfiguration,
}

#[derive(Serialize)]
#[serde(tag = "strategy", rename_all = "camelCase")]
enum HeadsDirectoryPolicy {
    Sibling { suffix: String },
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
#[serde(rename_all = "camelCase")]
struct ProjectLocator {
    version: u32,
    project_id: String,
    installation_id: String,
    project_root: String,
    heads_directory: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryMarker {
    version: u32,
    project_id: String,
    installation_id: String,
}

#[derive(Serialize)]
struct HeadInventory {
    version: u32,
    heads: BTreeMap<String, serde_json::Value>,
}

pub(super) struct InitialMetadata {
    pub(super) configuration: Vec<u8>,
    pub(super) locator: Vec<u8>,
    pub(super) marker: Vec<u8>,
    pub(super) inventory: Vec<u8>,
}

pub(super) fn serialize_initial_metadata(
    repository_name: &str,
    repository_root: &Path,
    heads_directory: &Path,
) -> Result<InitialMetadata, InitError> {
    let project_id = generate_project_id(repository_name);
    let installation_id = format!("local-{}", Uuid::new_v4().simple());
    let project_root = path_as_json_string(repository_root)?;
    let heads_directory = path_as_json_string(heads_directory)?;

    let configuration = serialize_project_configuration(&project_id)?;
    let locator = serialize_json(&ProjectLocator {
        version: 1,
        project_id: project_id.clone(),
        installation_id: installation_id.clone(),
        project_root,
        heads_directory,
    })?;
    let marker = serialize_json(&DirectoryMarker {
        version: 1,
        project_id,
        installation_id,
    })?;
    let inventory = serialize_json(&HeadInventory {
        version: 1,
        heads: BTreeMap::new(),
    })?;

    Ok(InitialMetadata {
        configuration,
        locator,
        marker,
        inventory,
    })
}

pub(super) fn serialize_project_configuration(project_id: &str) -> Result<Vec<u8>, InitError> {
    serialize_json(&ProjectConfiguration {
        version: 2,
        project_id: project_id.to_owned(),
        heads_directory: HeadsDirectoryPolicy::Sibling {
            suffix: ".heads".to_owned(),
        },
        branch_prefix: "hydra/".to_owned(),
        storage: StorageConfiguration {
            mode: "auto".to_owned(),
        },
        overlay: OverlayConfiguration {
            copy: vec!["... .gitignore".to_owned()],
        },
    })
}

fn path_as_json_string(path: &Path) -> Result<String, InitError> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| InitError::UnsupportedRepositoryPath(path.to_path_buf()))
}

fn generate_project_id(repository_name: &str) -> String {
    let mut slug = repository_name
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
    let suffix = Uuid::new_v4().simple();
    format!("{slug}-{suffix}")
}

fn serialize_json<T: Serialize>(value: &T) -> Result<Vec<u8>, InitError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(InitError::SerializeConfiguration)?;
    bytes.push(b'\n');
    Ok(bytes)
}

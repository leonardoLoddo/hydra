use std::{collections::BTreeMap, path::Path};

use serde::Serialize;
use uuid::Uuid;

use super::InitError;

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

pub(super) fn serialize_initial_configuration(
    repository_name: &str,
    heads_name: &str,
) -> Result<Vec<u8>, InitError> {
    serialize_json(&ProjectConfiguration {
        version: 1,
        project_id: generate_project_id(repository_name),
        heads_directory: Path::new("..")
            .join(heads_name)
            .to_string_lossy()
            .into_owned(),
        branch_prefix: "hydra/".to_owned(),
        storage: StorageConfiguration {
            mode: "auto".to_owned(),
        },
        overlay: OverlayConfiguration {
            copy: vec!["... .gitignore".to_owned()],
        },
    })
}

pub(super) fn serialize_initial_state() -> Result<Vec<u8>, InitError> {
    serialize_json(&LocalState {
        version: 1,
        heads: BTreeMap::new(),
    })
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

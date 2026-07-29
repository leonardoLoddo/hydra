use std::{fs, path::PathBuf};

const SCHEMA_URI: &str =
    "https://raw.githubusercontent.com/leonardoLoddo/hydra/main/schemas/v2/hydra.schema.json";

#[test]
fn version_two_schema_is_a_documented_strict_json_schema() {
    let schema_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/v2/hydra.schema.json");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(&schema_path).expect("the version-two schema should exist"),
    )
    .expect("the version-two schema should be valid JSON");

    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(schema["$id"], SCHEMA_URI);
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);

    let required = schema["required"]
        .as_array()
        .expect("top-level required fields should be declared");
    for name in [
        "version",
        "projectId",
        "headsDirectory",
        "branchPrefix",
        "storage",
        "overlay",
    ] {
        assert!(
            required.iter().any(|candidate| candidate == name),
            "{name} should be required"
        );
    }

    let properties = schema["properties"]
        .as_object()
        .expect("top-level properties should be declared");
    for name in [
        "$schema",
        "version",
        "projectId",
        "headsDirectory",
        "branchPrefix",
        "storage",
        "overlay",
        "commands",
    ] {
        let property = properties
            .get(name)
            .unwrap_or_else(|| panic!("{name} should be described by the schema"));
        assert!(
            property["description"]
                .as_str()
                .is_some_and(|description| !description.is_empty()),
            "{name} should include editor-facing help"
        );
    }
}

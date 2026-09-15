mod common;

use std::{fs, path::Path, process::Command};

use common::{TestDirectory, hydra_command};

fn assert_single_line_json_output(output: &[u8]) {
    let (terminator, content) = output
        .split_last()
        .expect("JSON output should contain a value and a newline");
    assert_eq!(*terminator, b'\n');
    assert!(!content.contains(&b'\n'));
}

fn isolated_hydra(home: &Path) -> Command {
    let mut command = hydra_command();
    command.env("HOME", home).env_remove("USERPROFILE");
    command
}

fn skill_path(home: &Path) -> std::path::PathBuf {
    home.join(".agents/skills/hydra")
}

fn gemini_skill_path(home: &Path) -> std::path::PathBuf {
    home.join(".gemini/skills/hydra")
}

fn agy_skill_path(home: &Path) -> std::path::PathBuf {
    home.join(".gemini/antigravity-cli/skills/hydra")
}

fn antigravity_skill_path(home: &Path) -> std::path::PathBuf {
    home.join(".gemini/config/skills/hydra")
}

fn install_skill(home: &Path) {
    let output = isolated_hydra(home)
        .args(["skill", "install", "codex", "--yes"])
        .output()
        .expect("Hydra CLI should start");
    assert!(
        output.status.success(),
        "install should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn skill_help_exposes_the_supported_provider_lifecycle() {
    let output = hydra_command()
        .args(["skill", "--help"])
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "skill help should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");
    for command in ["install", "status", "update", "remove"] {
        assert!(
            stdout.contains(command),
            "skill help should list {command:?}, got: {stdout:?}"
        );
    }
    for provider in ["codex", "gemini", "agy", "antigravity"] {
        assert!(
            stdout.contains(provider),
            "skill help should list {provider:?}, got: {stdout:?}"
        );
    }
}

#[test]
fn codex_install_yes_publishes_the_canonical_skill_and_provenance() {
    let directory = TestDirectory::new("skill-install");
    let output = isolated_hydra(directory.path())
        .args(["skill", "install", "codex", "--yes"])
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "install should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let destination = skill_path(directory.path());
    assert_eq!(
        fs::read(destination.join("SKILL.md")).expect("installed SKILL.md should exist"),
        include_bytes!("../../../skills/hydra/SKILL.md")
    );
    assert_eq!(
        fs::read(destination.join("agents/openai.yaml"))
            .expect("installed OpenAI metadata should exist"),
        include_bytes!("../../../skills/hydra/agents/openai.yaml")
    );
    for reference in ["arena.md", "augury.md", "gauntlet.md"] {
        assert!(
            destination.join("references").join(reference).is_file(),
            "installed skill should include references/{reference}"
        );
    }
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(destination.join(".hydra-skill.json")).expect("provenance manifest should exist"),
    )
    .expect("provenance manifest should be valid JSON");
    assert_eq!(manifest["schemaVersion"], 1);
    assert_eq!(manifest["provider"], "codex");
    assert_eq!(manifest["hydraVersion"], env!("CARGO_PKG_VERSION"));
    assert!(manifest["files"]["SKILL.md"].as_str().is_some());
    assert!(manifest["files"]["agents/openai.yaml"].as_str().is_some());
    for reference in ["arena.md", "augury.md", "gauntlet.md"] {
        assert!(
            manifest["files"][format!("references/{reference}")]
                .as_str()
                .is_some(),
            "provenance should include references/{reference}"
        );
    }
    assert!(
        !directory.path().join(".codex/skills/hydra").exists(),
        "the obsolete Codex skill location must not be used"
    );
    let stdout = String::from_utf8(output.stdout).expect("install output should be UTF-8");
    assert!(stdout.contains(&destination.display().to_string()));
}

#[test]
fn gemini_lifecycle_uses_its_destination_and_provider_provenance() {
    let directory = TestDirectory::new("gemini-skill-install");
    let output = isolated_hydra(directory.path())
        .args(["skill", "install", "gemini", "--yes"])
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "install should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let destination = gemini_skill_path(directory.path());
    assert_eq!(
        fs::read(destination.join("SKILL.md")).expect("installed SKILL.md should exist"),
        include_bytes!("../../../skills/hydra/SKILL.md")
    );
    assert_eq!(
        fs::read(destination.join("agents/openai.yaml"))
            .expect("installed OpenAI metadata should exist"),
        include_bytes!("../../../skills/hydra/agents/openai.yaml")
    );
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(destination.join(".hydra-skill.json")).expect("provenance manifest should exist"),
    )
    .expect("provenance manifest should be valid JSON");
    assert_eq!(manifest["provider"], "gemini");
    assert!(String::from_utf8_lossy(&output.stdout).contains("Gemini CLI"));

    let status = isolated_hydra(directory.path())
        .args(["skill", "status", "gemini"])
        .output()
        .expect("Hydra CLI should start");
    assert!(status.status.success());
    assert!(String::from_utf8_lossy(&status.stdout).contains("current"));

    let manifest_path = destination.join(".hydra-skill.json");
    let mut older_manifest = manifest;
    older_manifest["hydraVersion"] = serde_json::Value::String("0.0.0".to_owned());
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&older_manifest).expect("manifest should serialize"),
    )
    .expect("older manifest should be written");
    let update = isolated_hydra(directory.path())
        .args(["skill", "update", "gemini", "--yes"])
        .output()
        .expect("Hydra CLI should start");
    assert!(update.status.success());

    let remove = isolated_hydra(directory.path())
        .args(["skill", "remove", "gemini", "--yes"])
        .output()
        .expect("Hydra CLI should start");
    assert!(remove.status.success());
    assert!(!destination.exists());
}

#[test]
fn agy_lifecycle_uses_its_destination_and_provider_provenance() {
    let directory = TestDirectory::new("agy-skill-install");
    let output = isolated_hydra(directory.path())
        .args(["skill", "install", "agy", "--yes"])
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "install should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let destination = agy_skill_path(directory.path());
    assert_eq!(
        fs::read(destination.join("SKILL.md")).expect("installed SKILL.md should exist"),
        include_bytes!("../../../skills/hydra/SKILL.md")
    );
    assert_eq!(
        fs::read(destination.join("agents/openai.yaml"))
            .expect("installed OpenAI metadata should exist"),
        include_bytes!("../../../skills/hydra/agents/openai.yaml")
    );
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(destination.join(".hydra-skill.json")).expect("provenance manifest should exist"),
    )
    .expect("provenance manifest should be valid JSON");
    assert_eq!(manifest["provider"], "agy");
    assert!(String::from_utf8_lossy(&output.stdout).contains("Antigravity CLI"));

    let status = isolated_hydra(directory.path())
        .args(["skill", "status", "agy"])
        .output()
        .expect("Hydra CLI should start");
    assert!(status.status.success());
    assert!(String::from_utf8_lossy(&status.stdout).contains("current"));

    let manifest_path = destination.join(".hydra-skill.json");
    let mut older_manifest = manifest;
    older_manifest["hydraVersion"] = serde_json::Value::String("0.0.0".to_owned());
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&older_manifest).expect("manifest should serialize"),
    )
    .expect("older manifest should be written");
    let update = isolated_hydra(directory.path())
        .args(["skill", "update", "agy", "--yes"])
        .output()
        .expect("Hydra CLI should start");
    assert!(update.status.success());

    let remove = isolated_hydra(directory.path())
        .args(["skill", "remove", "agy", "--yes"])
        .output()
        .expect("Hydra CLI should start");
    assert!(remove.status.success());
    assert!(!destination.exists());
}

#[test]
fn antigravity_lifecycle_uses_its_destination_and_provider_provenance() {
    let directory = TestDirectory::new("antigravity-skill-install");
    let output = isolated_hydra(directory.path())
        .args(["skill", "install", "antigravity", "--yes"])
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "install should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let destination = antigravity_skill_path(directory.path());
    assert_eq!(
        fs::read(destination.join("SKILL.md")).expect("installed SKILL.md should exist"),
        include_bytes!("../../../skills/hydra/SKILL.md")
    );
    assert_eq!(
        fs::read(destination.join("agents/openai.yaml"))
            .expect("installed OpenAI metadata should exist"),
        include_bytes!("../../../skills/hydra/agents/openai.yaml")
    );
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(destination.join(".hydra-skill.json")).expect("provenance manifest should exist"),
    )
    .expect("provenance manifest should be valid JSON");
    assert_eq!(manifest["provider"], "antigravity");
    assert!(String::from_utf8_lossy(&output.stdout).contains("Antigravity app"));

    let manifest_path = destination.join(".hydra-skill.json");
    let mut mismatched_manifest = manifest.clone();
    mismatched_manifest["provider"] = serde_json::Value::String("gemini".to_owned());
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&mismatched_manifest).expect("manifest should serialize"),
    )
    .expect("mismatched manifest should be written");
    let mismatched_update = isolated_hydra(directory.path())
        .args(["skill", "update", "antigravity", "--yes"])
        .output()
        .expect("Hydra CLI should start");
    assert!(!mismatched_update.status.success());
    assert!(
        String::from_utf8_lossy(&mismatched_update.stderr)
            .contains("not owned by the supported Antigravity app adapter")
    );
    assert_eq!(
        fs::read(&manifest_path).expect("mismatched manifest should be preserved"),
        serde_json::to_vec_pretty(&mismatched_manifest).expect("manifest should serialize")
    );
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("managed manifest should be restored");

    let status = isolated_hydra(directory.path())
        .args(["skill", "status", "antigravity"])
        .output()
        .expect("Hydra CLI should start");
    assert!(status.status.success());
    assert!(String::from_utf8_lossy(&status.stdout).contains("current"));

    let mut older_manifest = manifest;
    older_manifest["hydraVersion"] = serde_json::Value::String("0.0.0".to_owned());
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&older_manifest).expect("manifest should serialize"),
    )
    .expect("older manifest should be written");
    let update = isolated_hydra(directory.path())
        .args(["skill", "update", "antigravity", "--yes"])
        .output()
        .expect("Hydra CLI should start");
    assert!(update.status.success());

    let remove = isolated_hydra(directory.path())
        .args(["skill", "remove", "antigravity", "--yes"])
        .output()
        .expect("Hydra CLI should start");
    assert!(remove.status.success());
    assert!(!destination.exists());
}

#[test]
fn codex_install_without_a_terminal_or_explicit_choice_skips_safely() {
    let directory = TestDirectory::new("skill-skip");
    let output = isolated_hydra(directory.path())
        .args(["skill", "install", "codex"])
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    assert!(!skill_path(directory.path()).exists());
    assert!(String::from_utf8_lossy(&output.stdout).contains("not installed"));
}

#[test]
fn codex_install_no_skips_safely() {
    let directory = TestDirectory::new("skill-no");
    let output = isolated_hydra(directory.path())
        .args(["skill", "install", "codex", "--no"])
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    assert!(!skill_path(directory.path()).exists());
}

#[test]
fn codex_install_preserves_an_existing_unknown_skill() {
    let directory = TestDirectory::new("skill-existing");
    let destination = skill_path(directory.path());
    fs::create_dir_all(&destination).expect("existing skill directory should be created");
    fs::write(destination.join("SKILL.md"), b"local skill\n")
        .expect("existing skill should be written");

    let output = isolated_hydra(directory.path())
        .args(["skill", "install", "codex", "--yes"])
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert_eq!(
        fs::read(destination.join("SKILL.md")).expect("existing skill should remain"),
        b"local skill\n"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
    assert!(stderr.contains("next: Run `hydra skill status <PROVIDER>`"));
}

#[test]
fn explicit_no_does_not_inspect_or_mutate_existing_skill_content() {
    let directory = TestDirectory::new("skill-existing-no");
    let destination = skill_path(directory.path());
    fs::create_dir_all(&destination).expect("existing skill directory should be created");
    fs::write(destination.join("SKILL.md"), b"local skill\n")
        .expect("existing skill should be written");

    for action in ["install", "update", "remove"] {
        let output = isolated_hydra(directory.path())
            .args(["skill", action, "codex", "--no"])
            .output()
            .expect("Hydra CLI should start");
        assert!(
            output.status.success(),
            "explicitly declining {action} should succeed, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read(destination.join("SKILL.md")).expect("existing skill should remain"),
            b"local skill\n"
        );
    }
}

#[test]
fn codex_status_reports_a_current_managed_installation() {
    let directory = TestDirectory::new("skill-status");
    install_skill(directory.path());

    let output = isolated_hydra(directory.path())
        .args(["skill", "status", "codex"])
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("current"));
}

#[test]
fn codex_status_emits_current_and_update_available_json() {
    let directory = TestDirectory::new("skill-status-json");
    install_skill(directory.path());
    let destination = skill_path(directory.path());

    let current = isolated_hydra(directory.path())
        .args(["skill", "status", "codex", "--json"])
        .output()
        .expect("Hydra CLI should start");
    assert!(
        current.status.success(),
        "JSON skill status should succeed, stderr: {}",
        String::from_utf8_lossy(&current.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&current.stdout)
            .expect("skill status should be valid JSON"),
        serde_json::json!({
            "schemaVersion": 1,
            "provider": "codex",
            "displayName": "Codex",
            "destination": destination.to_str().expect("test path should be Unicode"),
            "state": "current",
            "installedHydraVersion": env!("CARGO_PKG_VERSION"),
            "availableHydraVersion": env!("CARGO_PKG_VERSION")
        })
    );
    assert_single_line_json_output(&current.stdout);

    let manifest_path = destination.join(".hydra-skill.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("manifest should exist"))
            .expect("manifest should be JSON");
    manifest["hydraVersion"] = serde_json::Value::String("0.0.0".to_owned());
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("older manifest should be written");

    let outdated = isolated_hydra(directory.path())
        .args(["skill", "status", "codex", "--json"])
        .output()
        .expect("Hydra CLI should start");
    assert!(outdated.status.success());
    let json: serde_json::Value =
        serde_json::from_slice(&outdated.stdout).expect("skill status should be valid JSON");
    assert_eq!(json["state"], "updateAvailable");
    assert_eq!(json["installedHydraVersion"], "0.0.0");
    assert_eq!(json["availableHydraVersion"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn json_skill_status_errors_keep_stdout_empty_and_human_guidance_on_stderr() {
    let directory = TestDirectory::new("skill-status-json-absent");

    let output = isolated_hydra(directory.path())
        .args(["skill", "status", "codex", "--json"])
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("error output should be UTF-8");
    assert!(stderr.contains("is not installed"));
    assert!(stderr.contains("next: Run `hydra skill install <PROVIDER>`"));
}

#[test]
fn codex_update_refreshes_provenance_for_an_older_managed_version() {
    let directory = TestDirectory::new("skill-update");
    install_skill(directory.path());
    let manifest_path = skill_path(directory.path()).join(".hydra-skill.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("manifest should exist"))
            .expect("manifest should be JSON");
    manifest["hydraVersion"] = serde_json::Value::String("0.0.0".to_owned());
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("older manifest should be written");

    let output = isolated_hydra(directory.path())
        .args(["skill", "update", "codex", "--yes"])
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "update should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let updated: serde_json::Value =
        serde_json::from_slice(&fs::read(manifest_path).expect("manifest should remain"))
            .expect("updated manifest should be JSON");
    assert_eq!(updated["hydraVersion"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn codex_update_migrates_the_legacy_two_file_skill_layout() {
    let directory = TestDirectory::new("skill-update-legacy-layout");
    install_skill(directory.path());
    let destination = skill_path(directory.path());
    fs::remove_dir_all(destination.join("references"))
        .expect("current references should be removed to emulate the legacy layout");
    let manifest_path = destination.join(".hydra-skill.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("manifest should exist"))
            .expect("manifest should be JSON");
    manifest["hydraVersion"] = serde_json::Value::String("1.0.0".to_owned());
    let files = manifest["files"]
        .as_object_mut()
        .expect("manifest files should be an object");
    files.retain(|relative, _| relative == "SKILL.md" || relative == "agents/openai.yaml");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("legacy manifest should be written");

    let output = isolated_hydra(directory.path())
        .args(["skill", "update", "codex", "--yes"])
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "legacy managed layout should update, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    for reference in ["arena.md", "augury.md", "gauntlet.md"] {
        assert!(destination.join("references").join(reference).is_file());
    }
}

#[test]
fn codex_update_and_remove_preserve_locally_modified_content() {
    let directory = TestDirectory::new("skill-modified");
    install_skill(directory.path());
    let destination = skill_path(directory.path());
    fs::write(destination.join("SKILL.md"), b"locally modified\n")
        .expect("installed skill should be changed");

    for action in ["update", "remove"] {
        let output = isolated_hydra(directory.path())
            .args(["skill", action, "codex", "--yes"])
            .output()
            .expect("Hydra CLI should start");
        assert!(!output.status.success(), "{action} must refuse changes");
        assert!(String::from_utf8_lossy(&output.stderr).contains("modified"));
        assert_eq!(
            fs::read(destination.join("SKILL.md")).expect("modified skill should remain"),
            b"locally modified\n"
        );
    }
}

#[test]
fn codex_update_and_remove_preserve_a_locally_modified_art_reference() {
    let directory = TestDirectory::new("skill-modified-reference");
    install_skill(directory.path());
    let destination = skill_path(directory.path());
    let reference = destination.join("references/arena.md");
    fs::write(&reference, b"locally modified\n")
        .expect("installed Art reference should be changed");

    for action in ["update", "remove"] {
        let output = isolated_hydra(directory.path())
            .args(["skill", action, "codex", "--yes"])
            .output()
            .expect("Hydra CLI should start");
        assert!(!output.status.success(), "{action} must refuse changes");
        assert!(String::from_utf8_lossy(&output.stderr).contains("modified"));
        assert_eq!(
            fs::read(&reference).expect("modified Art reference should remain"),
            b"locally modified\n"
        );
    }
}

#[test]
fn codex_remove_deletes_only_an_unmodified_managed_installation() {
    let directory = TestDirectory::new("skill-remove");
    install_skill(directory.path());

    let output = isolated_hydra(directory.path())
        .args(["skill", "remove", "codex", "--yes"])
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "remove should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!skill_path(directory.path()).exists());
}

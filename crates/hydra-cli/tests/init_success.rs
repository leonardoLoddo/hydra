mod common;

use std::{fs, path::Path, process::Command};

use common::{
    TestDirectory, copy_on_write_guidance_url, create_initialized_project, heads_directory,
    hydra_command,
};

fn initialize_repository(path: &Path) {
    let git = Command::new("git")
        .args(["init", "--quiet"])
        .arg(path)
        .status()
        .expect("Git should start");
    assert!(git.success(), "temporary Git repository should be created");
}

fn assert_full_copy_guidance(stdout: &str) {
    let Some(guidance) = copy_on_write_guidance_url() else {
        return;
    };
    let guidance = format!("Copy-on-write guidance: {guidance}\n");
    if stdout.contains("Storage backend: full copy\n") {
        assert!(stdout.contains(&guidance));
    } else {
        assert!(!stdout.contains(&guidance));
    }
}

#[test]
fn init_creates_project_configuration_heads_directory_and_local_state() {
    let directory = TestDirectory::new("success");
    let repository = directory.path().join("SampleProject");
    fs::create_dir(&repository).expect("repository directory should be created");
    initialize_repository(&repository);

    let output = hydra_command()
        .arg("init")
        .arg(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "initialization should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("success output should be UTF-8");
    assert!(
        stdout.contains("Storage backend: copy-on-write")
            || stdout.contains("Storage backend: full copy"),
        "initialization should report the verified storage backend, got: {stdout:?}"
    );
    assert_full_copy_guidance(&stdout);

    let config: serde_json::Value = serde_json::from_slice(
        &fs::read(repository.join(".hydra.json")).expect("project configuration should be created"),
    )
    .expect("project configuration should be valid JSON");

    assert!(config.get("$schema").is_none(), "$schema must be absent");
    assert_eq!(config["version"], 2);
    assert_eq!(
        config["headsDirectory"],
        serde_json::json!({
            "strategy": "sibling",
            "suffix": ".heads"
        })
    );
    assert_eq!(config["branchPrefix"], "hydra/");
    assert_eq!(config["storage"]["mode"], "auto");
    assert_eq!(config["overlay"]["copy"][0], "... .gitignore");
    assert!(
        config["projectId"].as_str().is_some_and(|project_id| {
            project_id
                .strip_prefix("sampleproject-")
                .is_some_and(|suffix| {
                    suffix.len() == 32 && suffix.chars().all(|c| c.is_ascii_hexdigit())
                })
        }),
        "project ID should contain the repository slug and the complete UUID entropy"
    );

    let heads_directory = common::canonical_path(directory.path().join("SampleProject.heads"))
        .expect("default sibling Heads directory should resolve");
    assert!(
        heads_directory.is_dir(),
        "default sibling Heads directory should be created"
    );

    let locator: serde_json::Value = serde_json::from_slice(
        &fs::read(repository.join(".git/hydra/project.json"))
            .expect("local project locator should be created"),
    )
    .expect("local project locator should be valid JSON");
    assert_eq!(locator["version"], 1);
    assert_eq!(locator["projectId"], config["projectId"]);
    assert_eq!(
        locator["projectRoot"],
        common::canonical_path(&repository)
            .expect("repository should resolve")
            .display()
            .to_string()
    );
    assert_eq!(
        locator["headsDirectory"],
        heads_directory.display().to_string()
    );
    assert!(
        locator["installationId"]
            .as_str()
            .and_then(|value| value.strip_prefix("local-"))
            .is_some_and(|suffix| {
                suffix.len() == 32 && suffix.chars().all(|c| c.is_ascii_hexdigit())
            }),
        "installation ID should contain complete UUID entropy"
    );

    let marker: serde_json::Value = serde_json::from_slice(
        &fs::read(heads_directory.join(".hydra/directory.json"))
            .expect("Heads directory ownership marker should be created"),
    )
    .expect("ownership marker should be valid JSON");
    assert_eq!(marker["version"], 1);
    assert_eq!(marker["projectId"], locator["projectId"]);
    assert_eq!(marker["installationId"], locator["installationId"]);

    let state: serde_json::Value = serde_json::from_slice(
        &fs::read(heads_directory.join(".hydra/heads.json"))
            .expect("local Head inventory should be created"),
    )
    .expect("local Head inventory should be valid JSON");
    assert_eq!(state["version"], 1);
    assert_eq!(state["heads"], serde_json::json!({}));
    assert!(
        !repository.join(".git/hydra/heads.json").exists(),
        "the Git common directory should contain only the local locator"
    );
}

#[test]
fn init_defaults_to_the_current_directory() {
    let directory = TestDirectory::new("default-path");
    let repository = directory.path().join("SampleProject");
    fs::create_dir(&repository).expect("repository directory should be created");
    initialize_repository(&repository);

    let output = hydra_command()
        .arg("init")
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "default path should initialize the current repository, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(repository.join(".hydra.json").is_file());
    assert!(directory.path().join("SampleProject.heads").is_dir());
    assert!(repository.join(".git/hydra/project.json").is_file());
    assert!(
        directory
            .path()
            .join("SampleProject.heads/.hydra/heads.json")
            .is_file()
    );
}

#[test]
fn init_recovers_configuration_for_the_same_owned_heads_directory() {
    let directory = TestDirectory::new("recover-owned-heads");
    let repository = create_initialized_project(&directory);

    let locator_path = repository.join(".git/hydra/project.json");
    let marker_path = heads_directory(&repository).join(".hydra/directory.json");
    let inventory_path = heads_directory(&repository).join(".hydra/heads.json");
    let locator_before = fs::read(&locator_path).expect("locator should be readable");
    let marker_before = fs::read(&marker_path).expect("marker should be readable");
    let inventory_before = fs::read(&inventory_path).expect("inventory should be readable");
    let locator: serde_json::Value =
        serde_json::from_slice(&locator_before).expect("locator should be valid JSON");
    fs::remove_file(repository.join(".hydra.json"))
        .expect("configuration loss should be simulated");

    let output = hydra_command()
        .arg("init")
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "owned directory recovery should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let configuration: serde_json::Value = serde_json::from_slice(
        &fs::read(repository.join(".hydra.json")).expect("configuration should be reconstructed"),
    )
    .expect("configuration should be valid JSON");
    assert_eq!(configuration["projectId"], locator["projectId"]);
    assert_eq!(
        fs::read(&locator_path).expect("locator should remain readable"),
        locator_before
    );
    assert_eq!(
        fs::read(&marker_path).expect("marker should remain readable"),
        marker_before
    );
    assert_eq!(
        fs::read(&inventory_path).expect("inventory should remain readable"),
        inventory_before
    );
    assert!(heads_directory(&repository).is_dir());

    let status = hydra_command()
        .arg("status")
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");
    assert!(
        status.status.success(),
        "recovered installation should be usable, stderr: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

#[test]
fn init_uses_the_shared_git_directory_from_a_linked_worktree() {
    let directory = TestDirectory::new("linked-worktree");
    let repository = directory.path().join("MainProject");
    let linked_worktree = directory.path().join("LinkedHead");

    initialize_repository(&repository);
    fs::write(repository.join("tracked.txt"), b"base\n").expect("fixture should be written");
    for arguments in [
        vec![
            "-C",
            repository.to_str().expect("path should be UTF-8"),
            "add",
            ".",
        ],
        vec![
            "-C",
            repository.to_str().expect("path should be UTF-8"),
            "-c",
            "user.name=Hydra Tests",
            "-c",
            "user.email=hydra-tests@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ],
    ] {
        let status = Command::new("git")
            .args(arguments)
            .status()
            .expect("Git should start");
        assert!(status.success(), "Git fixture command should succeed");
    }
    let status = Command::new("git")
        .arg("-C")
        .arg(&repository)
        .args(["worktree", "add", "--quiet", "-b", "linked-head"])
        .arg(&linked_worktree)
        .status()
        .expect("Git should start");
    assert!(status.success(), "linked worktree should be created");

    let output = hydra_command()
        .arg("init")
        .arg(&linked_worktree)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "linked worktree initialization should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(linked_worktree.join(".hydra.json").is_file());
    assert!(directory.path().join("LinkedHead.heads").is_dir());
    assert!(
        repository.join(".git/hydra/project.json").is_file(),
        "the local locator must use the common Git directory"
    );
    assert!(
        !linked_worktree.join(".git/hydra/project.json").exists(),
        "Hydra must not treat the linked worktree .git file as a directory"
    );
    assert!(
        directory
            .path()
            .join("LinkedHead.heads/.hydra/heads.json")
            .is_file(),
        "the physical inventory must live in the Heads directory"
    );
}

#[cfg(unix)]
#[test]
fn init_preserves_trailing_whitespace_in_the_repository_path() {
    let directory = TestDirectory::new("trailing-space");
    let repository = directory.path().join("SampleProject ");
    fs::create_dir(&repository).expect("repository directory should be created");
    initialize_repository(&repository);

    let output = hydra_command()
        .arg("init")
        .arg(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "valid repository path should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        repository.join(".hydra.json").is_file(),
        "configuration should be created in the exact repository path"
    );
    assert!(
        directory.path().join("SampleProject .heads").is_dir(),
        "Heads directory should preserve repository-name whitespace"
    );
}

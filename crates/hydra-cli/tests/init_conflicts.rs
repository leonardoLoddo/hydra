mod common;

use std::{fs, path::Path, process::Command};

#[cfg(unix)]
use std::os::unix::fs::symlink;

use common::{TestDirectory, create_initialized_project, heads_directory, hydra_command};

fn initialize_repository(path: &Path) {
    let git = Command::new("git")
        .args(["init", "--quiet"])
        .arg(path)
        .status()
        .expect("Git should start");
    assert!(git.success(), "temporary Git repository should be created");
}

#[test]
fn init_rejects_a_directory_outside_a_git_repository() {
    let directory = TestDirectory::new("not-git");

    let output = hydra_command()
        .arg("init")
        .arg(directory.path())
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("error output should be UTF-8");
    assert!(
        stderr.to_ascii_lowercase().contains("not a git repository"),
        "error should identify the invalid directory, got: {stderr:?}"
    );
    assert!(
        !directory.path().join(".hydra.json").exists(),
        "failed initialization must not create project configuration"
    );
}

#[test]
fn init_from_a_head_reports_the_canonical_parent_as_already_initialized() {
    let directory = TestDirectory::new("init-from-head");
    let repository = create_initialized_project(&directory);
    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");
    assert!(output.status.success());
    let head = heads_directory(&repository).join("payment");

    let output = hydra_command()
        .arg("init")
        .current_dir(&head)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let parent_configuration = common::canonical_path(&repository)
        .expect("parent repository should resolve")
        .join(".hydra.json");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "error: Hydra is already initialized at {}\n",
            parent_configuration.display()
        )
    );
    assert!(!head.join(".hydra.json").exists());
}

#[test]
fn init_refuses_to_reuse_a_preexisting_default_heads_directory() {
    let directory = TestDirectory::new("heads-exist");
    let repository = directory.path().join("SampleProject");
    let heads_directory = directory.path().join("SampleProject.heads");
    fs::create_dir(&repository).expect("repository directory should be created");
    fs::create_dir(&heads_directory).expect("preexisting Heads directory should be created");
    initialize_repository(&repository);

    let output = hydra_command()
        .arg("init")
        .arg(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("already exists"),
        "error should explain the destination conflict"
    );
    assert!(
        !repository.join(".hydra.json").exists(),
        "destination conflict must not create project configuration"
    );
    assert!(
        !repository.join(".git/hydra/project.json").exists(),
        "destination conflict must not create the local locator"
    );
}

#[test]
fn init_rejects_existing_heads_when_locator_and_marker_ownership_disagree() {
    let directory = TestDirectory::new("owned-heads-identity-mismatch");
    let repository = create_initialized_project(&directory);
    let marker_path = heads_directory(&repository).join(".hydra/directory.json");
    let mut marker: serde_json::Value = serde_json::from_slice(
        &fs::read(&marker_path).expect("ownership marker should be readable"),
    )
    .expect("ownership marker should be valid JSON");
    marker["installationId"] = serde_json::json!("local-other-installation");
    let mut marker_bytes = serde_json::to_vec_pretty(&marker).expect("marker should serialize");
    marker_bytes.push(b'\n');
    fs::write(&marker_path, &marker_bytes).expect("fixture marker should be replaced");
    fs::remove_file(repository.join(".hydra.json"))
        .expect("configuration loss should be simulated");

    let output = hydra_command()
        .arg("init")
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("ownership metadata do not match"),
        "error should identify the ownership mismatch"
    );
    assert!(!repository.join(".hydra.json").exists());
    assert_eq!(
        fs::read(&marker_path).expect("marker should remain readable"),
        marker_bytes
    );
}

#[test]
fn init_rejects_an_incomplete_preexisting_owned_heads_directory() {
    let directory = TestDirectory::new("owned-heads-incomplete");
    let repository = create_initialized_project(&directory);
    let inventory_path = heads_directory(&repository).join(".hydra/heads.json");
    fs::remove_file(&inventory_path).expect("inventory loss should be simulated");
    fs::remove_file(repository.join(".hydra.json"))
        .expect("configuration loss should be simulated");

    let output = hydra_command()
        .arg("init")
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("existing Hydra installation is incomplete"),
        "error should identify incomplete recovery evidence"
    );
    assert!(!repository.join(".hydra.json").exists());
    assert!(!inventory_path.exists());
}

#[test]
fn init_does_not_guess_lost_configuration_for_an_owned_directory_with_heads() {
    let directory = TestDirectory::new("owned-heads-with-state");
    let repository = create_initialized_project(&directory);
    let create = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");
    assert!(create.status.success());
    let inventory_path = heads_directory(&repository).join(".hydra/heads.json");
    let inventory_before = fs::read(&inventory_path).expect("inventory should be readable");
    fs::remove_file(repository.join(".hydra.json"))
        .expect("configuration loss should be simulated");

    let output = hydra_command()
        .arg("init")
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("cannot safely reconstruct configuration for existing Heads"),
        "error should refuse to guess prior project policy"
    );
    assert!(!repository.join(".hydra.json").exists());
    assert_eq!(
        fs::read(&inventory_path).expect("inventory should remain readable"),
        inventory_before
    );
    assert!(heads_directory(&repository).join("payment").is_dir());
}

#[test]
fn init_does_not_reuse_an_owned_heads_directory_with_unreconciled_content() {
    let directory = TestDirectory::new("owned-heads-residue");
    let repository = create_initialized_project(&directory);
    let residue = heads_directory(&repository).join("interrupted-head");
    fs::create_dir(&residue).expect("unreconciled directory should be created");
    fs::write(residue.join("work.txt"), b"preserve me\n")
        .expect("unreconciled content should be created");
    fs::remove_file(repository.join(".hydra.json"))
        .expect("configuration loss should be simulated");

    let output = hydra_command()
        .arg("init")
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("cannot safely reconstruct configuration for existing Heads"),
        "error should refuse unreconciled content"
    );
    assert_eq!(
        fs::read(residue.join("work.txt")).expect("residue should remain readable"),
        b"preserve me\n"
    );
    assert!(!repository.join(".hydra.json").exists());
}

#[test]
fn init_rolls_back_the_heads_directory_when_local_state_cannot_be_created() {
    let directory = TestDirectory::new("state-failure");
    let repository = directory.path().join("SampleProject");
    fs::create_dir(&repository).expect("repository directory should be created");
    initialize_repository(&repository);

    fs::write(repository.join(".git/hydra"), b"state path conflict")
        .expect("conflicting state path should be created");

    let output = hydra_command()
        .arg("init")
        .arg(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        !repository.join(".hydra.json").exists(),
        "state failure must not publish project configuration"
    );
    assert!(
        !directory.path().join("SampleProject.heads").exists(),
        "state failure must roll back the newly created Heads directory"
    );
}

#[test]
fn init_refuses_to_claim_a_preexisting_local_state_directory() {
    let directory = TestDirectory::new("state-directory-exists");
    let repository = directory.path().join("SampleProject");
    fs::create_dir(&repository).expect("repository directory should be created");
    initialize_repository(&repository);
    let state_directory = repository.join(".git/hydra");
    fs::create_dir(&state_directory).expect("preexisting state directory should be created");

    let output = hydra_command()
        .arg("init")
        .arg(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("already exists"),
        "error should explain the ownership conflict"
    );
    assert!(
        state_directory.is_dir(),
        "Hydra must preserve the preexisting state directory"
    );
    assert!(
        !repository.join(".hydra.json").exists(),
        "ownership conflict must not publish project configuration"
    );
    assert!(
        !directory.path().join("SampleProject.heads").exists(),
        "ownership conflict must be detected before creating the Heads directory"
    );
}

#[cfg(unix)]
#[test]
fn init_rejects_a_symlinked_local_state_directory() {
    let directory = TestDirectory::new("state-symlink");
    let repository = directory.path().join("SampleProject");
    let external_state = directory.path().join("external-state");
    fs::create_dir(&repository).expect("repository directory should be created");
    fs::create_dir(&external_state).expect("external state directory should be created");
    initialize_repository(&repository);

    let state_directory = repository.join(".git/hydra");
    symlink(&external_state, &state_directory).expect("state symlink should be created");

    let output = hydra_command()
        .arg("init")
        .arg(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        fs::symlink_metadata(&state_directory)
            .expect("state symlink should remain")
            .file_type()
            .is_symlink(),
        "Hydra must not replace the state symlink"
    );
    assert!(
        !external_state.join("project.json").exists(),
        "Hydra must not write through the state symlink"
    );
    assert!(
        !repository.join(".hydra.json").exists(),
        "unsafe state path must not publish project configuration"
    );
    assert!(
        !directory.path().join("SampleProject.heads").exists(),
        "unsafe state path must not leave a Heads directory"
    );
}

#[cfg(unix)]
#[test]
fn init_preserves_a_dangling_project_configuration_symlink() {
    let directory = TestDirectory::new("config-symlink");
    let repository = directory.path().join("SampleProject");
    let missing_target = directory.path().join("missing-config-target");
    fs::create_dir(&repository).expect("repository directory should be created");
    initialize_repository(&repository);

    let configuration_path = repository.join(".hydra.json");
    symlink(&missing_target, &configuration_path)
        .expect("dangling configuration symlink should be created");

    let output = hydra_command()
        .arg("init")
        .arg(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        fs::symlink_metadata(&configuration_path)
            .expect("configuration symlink should remain")
            .file_type()
            .is_symlink(),
        "Hydra must not replace the configuration symlink"
    );
    assert!(
        !missing_target.exists(),
        "Hydra must not create the symlink target"
    );
    assert!(
        !repository.join(".git/hydra/project.json").exists(),
        "configuration conflict must not create the local locator"
    );
    assert!(
        !directory.path().join("SampleProject.heads").exists(),
        "configuration conflict must not create a Heads directory"
    );
}

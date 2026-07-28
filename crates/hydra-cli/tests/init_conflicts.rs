mod common;

use std::{fs, path::Path, process::Command};

#[cfg(unix)]
use std::os::unix::fs::symlink;

use common::{TestDirectory, hydra_command};

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
        !repository.join(".git/hydra/heads.json").exists(),
        "destination conflict must not create local state"
    );
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
        !external_state.join("heads.json").exists(),
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
        !repository.join(".git/hydra/heads.json").exists(),
        "configuration conflict must not create local state"
    );
    assert!(
        !directory.path().join("SampleProject.heads").exists(),
        "configuration conflict must not create a Heads directory"
    );
}

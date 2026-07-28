mod common;

use std::{
    fs,
    process::Stdio,
    thread,
    time::{Duration, Instant},
};

use common::{
    TestDirectory, assert_no_head_creation_artifacts, create_initialized_project, hydra_command,
    run_git,
};

#[test]
fn head_create_releases_the_lock_when_local_state_is_malformed() {
    let directory = TestDirectory::new("head-malformed-state");
    let repository = create_initialized_project(&directory);
    fs::write(repository.join(".git/hydra/heads.json"), b"{not-json\n")
        .expect("malformed state should be written");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Hydra state"));
    assert_no_head_creation_artifacts(&repository, "payment");
}

#[test]
fn head_create_rejects_a_newer_configuration_version_before_mutation() {
    let directory = TestDirectory::new("head-newer-config");
    let repository = create_initialized_project(&directory);
    let configuration_path = repository.join(".hydra.json");
    let mut configuration: serde_json::Value = serde_json::from_slice(
        &fs::read(&configuration_path).expect("configuration should be readable"),
    )
    .expect("configuration should be valid JSON");
    configuration["version"] = 2.into();
    fs::write(
        &configuration_path,
        serde_json::to_vec_pretty(&configuration).expect("configuration should serialize"),
    )
    .expect("newer configuration should be written");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("configuration version 2"));
    assert_no_head_creation_artifacts(&repository, "payment");
}

#[test]
fn head_create_rejects_an_unknown_storage_mode_before_mutation() {
    let directory = TestDirectory::new("head-storage-mode");
    let repository = create_initialized_project(&directory);
    let configuration_path = repository.join(".hydra.json");
    let mut configuration: serde_json::Value = serde_json::from_slice(
        &fs::read(&configuration_path).expect("configuration should be readable"),
    )
    .expect("configuration should be valid JSON");
    configuration["storage"]["mode"] = "hardlink".into();
    fs::write(
        &configuration_path,
        serde_json::to_vec_pretty(&configuration).expect("configuration should serialize"),
    )
    .expect("unsupported configuration should be written");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("configuration"));
    assert_no_head_creation_artifacts(&repository, "payment");
}

#[test]
fn head_create_rejects_a_newer_state_version_without_leaving_a_lock() {
    let directory = TestDirectory::new("head-newer-state");
    let repository = create_initialized_project(&directory);
    let state_path = repository.join(".git/hydra/heads.json");
    fs::write(&state_path, b"{\"version\":2,\"heads\":{}}\n")
        .expect("newer state should be written");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("state version 2"));
    assert_no_head_creation_artifacts(&repository, "payment");
}

#[test]
fn head_create_preserves_committed_artifacts_when_only_lock_cleanup_fails() {
    let directory = TestDirectory::new("head-lock-cleanup");
    let repository = create_initialized_project(&directory);
    let lock_path = repository.join(".git/hydra/heads.json.lock");

    let child = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Hydra CLI should start");

    let deadline = Instant::now() + Duration::from_secs(5);
    while !lock_path.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(1));
    }
    assert!(lock_path.is_file(), "Hydra should acquire the state lock");
    fs::remove_file(&lock_path).expect("the lock file should be replaceable for the fixture");
    fs::create_dir(&lock_path).expect("a directory should force lock cleanup to fail");

    let output = child.wait_with_output().expect("Hydra should finish");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Head was committed"),
        "error should report the post-commit cleanup failure, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let head_path = directory.path().join("SampleProject.heads/payment");
    assert!(
        head_path.is_dir(),
        "a committed worktree must not be rolled back"
    );
    let branch = run_git(
        &repository,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            "refs/heads/hydra/payment",
        ],
    );
    assert!(branch.status.success(), "a committed branch must remain");
    let state: serde_json::Value = serde_json::from_slice(
        &fs::read(repository.join(".git/hydra/heads.json")).expect("state should be readable"),
    )
    .expect("state should remain valid JSON");
    assert!(state["heads"]["payment"].is_object());
}

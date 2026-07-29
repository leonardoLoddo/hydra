mod common;

use std::{
    fs,
    process::Stdio,
    thread,
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::fs::symlink;

use common::{
    TestDirectory, assert_no_head_creation_artifacts, create_initialized_project,
    head_state_lock_path, head_state_path, heads_directory, hydra_command, run_git,
};

#[test]
fn head_create_releases_the_lock_when_local_state_is_malformed() {
    let directory = TestDirectory::new("head-malformed-state");
    let repository = create_initialized_project(&directory);
    fs::write(head_state_path(&repository), b"{not-json\n")
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
fn head_create_rejects_a_heads_directory_owned_by_another_installation() {
    let directory = TestDirectory::new("head-ownership-mismatch");
    let repository = create_initialized_project(&directory);
    let marker_path = heads_directory(&repository).join(".hydra/directory.json");
    let mut marker: serde_json::Value = serde_json::from_slice(
        &fs::read(&marker_path).expect("ownership marker should be readable"),
    )
    .expect("ownership marker should be valid JSON");
    marker["installationId"] = "local-another-installation".into();
    fs::write(
        &marker_path,
        serde_json::to_vec_pretty(&marker).expect("ownership marker should serialize"),
    )
    .expect("ownership marker should be replaced");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("ownership"),
        "error should identify the ownership mismatch, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_no_head_creation_artifacts(&repository, "payment");
}

#[cfg(unix)]
#[test]
fn head_create_rejects_a_symlinked_heads_metadata_directory() {
    let directory = TestDirectory::new("head-metadata-symlink");
    let repository = create_initialized_project(&directory);
    let heads = heads_directory(&repository);
    let metadata_directory = heads.join(".hydra");
    let external_metadata = directory.path().join("external-head-metadata");
    fs::rename(&metadata_directory, &external_metadata)
        .expect("metadata should be moved outside the Heads directory");
    symlink(&external_metadata, &metadata_directory)
        .expect("Heads metadata symlink should be created");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unsafe"),
        "error should reject the metadata symlink, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_no_head_creation_artifacts(&repository, "payment");
}

#[test]
fn head_create_rejects_a_suffix_that_contains_a_path_separator() {
    let directory = TestDirectory::new("head-unsafe-suffix");
    let repository = create_initialized_project(&directory);
    let configuration_path = repository.join(".hydra.json");
    let mut configuration: serde_json::Value = serde_json::from_slice(
        &fs::read(&configuration_path).expect("configuration should be readable"),
    )
    .expect("configuration should be valid JSON");
    configuration["headsDirectory"]["suffix"] = "../escaped".into();
    fs::write(
        &configuration_path,
        serde_json::to_vec_pretty(&configuration).expect("configuration should serialize"),
    )
    .expect("unsafe configuration should be written");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(
            "suffix must be a non-empty filename fragment without separators or control characters"
        ),
        "error should identify the invalid suffix, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_no_head_creation_artifacts(&repository, "payment");
}

#[test]
fn head_create_rejects_a_heads_directory_nested_inside_another_worktree() {
    let directory = TestDirectory::new("head-nested-directory");
    let repository = create_initialized_project(&directory);
    let original_heads = heads_directory(&repository);
    let first = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");
    assert!(first.status.success());

    let nested_heads = original_heads.join("payment/nested-heads");
    fs::create_dir_all(nested_heads.join(".hydra"))
        .expect("nested metadata directory should be created");
    fs::copy(
        original_heads.join(".hydra/directory.json"),
        nested_heads.join(".hydra/directory.json"),
    )
    .expect("matching ownership marker should be copied");
    fs::write(
        nested_heads.join(".hydra/heads.json"),
        b"{\"version\":1,\"heads\":{}}\n",
    )
    .expect("nested inventory should be written");

    let configuration_path = repository.join(".hydra.json");
    let mut configuration: serde_json::Value = serde_json::from_slice(
        &fs::read(&configuration_path).expect("configuration should be readable"),
    )
    .expect("configuration should be valid JSON");
    configuration["headsDirectory"] = serde_json::json!({"strategy": "local"});
    fs::write(
        &configuration_path,
        serde_json::to_vec_pretty(&configuration).expect("configuration should serialize"),
    )
    .expect("configuration should be updated");

    let locator_path = repository.join(".git/hydra/project.json");
    let mut locator: serde_json::Value =
        serde_json::from_slice(&fs::read(&locator_path).expect("locator should be readable"))
            .expect("locator should be valid JSON");
    locator["headsDirectory"] = fs::canonicalize(&nested_heads)
        .expect("nested directory should resolve")
        .display()
        .to_string()
        .into();
    fs::write(
        &locator_path,
        serde_json::to_vec_pretty(&locator).expect("locator should serialize"),
    )
    .expect("locator should be updated");

    let output = hydra_command()
        .args(["head", "create", "auth"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unsafe"),
        "error should reject a directory nested in a worktree, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_no_head_creation_artifacts(&repository, "auth");
}

#[test]
fn head_create_rejects_the_obsolete_version_one_configuration() {
    let directory = TestDirectory::new("head-obsolete-config");
    let repository = create_initialized_project(&directory);
    let configuration_path = repository.join(".hydra.json");
    let mut configuration: serde_json::Value = serde_json::from_slice(
        &fs::read(&configuration_path).expect("configuration should be readable"),
    )
    .expect("configuration should be valid JSON");
    configuration["version"] = 1.into();
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
    assert!(String::from_utf8_lossy(&output.stderr).contains("configuration version 1"));
    assert_no_head_creation_artifacts(&repository, "payment");
}

#[test]
fn head_create_still_rejects_unknown_top_level_configuration_fields() {
    let directory = TestDirectory::new("head-unknown-config-field");
    let repository = create_initialized_project(&directory);
    let configuration_path = repository.join(".hydra.json");
    let mut configuration: serde_json::Value = serde_json::from_slice(
        &fs::read(&configuration_path).expect("configuration should be readable"),
    )
    .expect("configuration should be valid JSON");
    configuration["comment"] = "not a supported JSON Schema annotation".into();
    fs::write(
        &configuration_path,
        serde_json::to_vec_pretty(&configuration).expect("configuration should serialize"),
    )
    .expect("configuration with an unknown field should be written");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown field `comment`"),
        "the error should identify the unsupported field, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
    let state_path = head_state_path(&repository);
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
    let lock_path = head_state_lock_path(&repository);

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
        &fs::read(head_state_path(&repository)).expect("state should be readable"),
    )
    .expect("state should remain valid JSON");
    assert!(state["heads"]["payment"].is_object());
}

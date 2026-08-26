mod common;

use std::fs;

use common::{
    TestDirectory, assert_no_head_creation_artifacts, create_initialized_project,
    head_state_lock_path, hydra_command, run_git,
};

#[test]
fn head_create_rejects_an_unsafe_name_before_mutation() {
    let directory = TestDirectory::new("head-invalid-name");
    let repository = create_initialized_project(&directory);

    let output = hydra_command()
        .args(["head", "create", "../escape"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid Head name"));
    assert!(!directory.path().join("escape").exists());
    assert_no_head_creation_artifacts(&repository, "escape");
}

#[test]
fn head_create_rejects_an_unknown_base_without_leaving_artifacts() {
    let directory = TestDirectory::new("head-invalid-ref");
    let repository = create_initialized_project(&directory);

    let output = hydra_command()
        .args(["head", "create", "payment", "--from", "missing-ref"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("resolving the base commit"));
    assert_no_head_creation_artifacts(&repository, "payment");
}

#[test]
fn head_create_identifies_an_unknown_target_without_leaving_artifacts() {
    let directory = TestDirectory::new("head-invalid-target");
    let repository = create_initialized_project(&directory);

    let output = hydra_command()
        .args(["head", "create", "payment", "--target", "missing-target"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("normalizing the target ref"),
        "error should identify target normalization, got: {stderr}"
    );
    assert!(!stderr.contains("normalizing the base ref"));
    assert_no_head_creation_artifacts(&repository, "payment");
}

#[test]
fn head_create_preserves_a_preexisting_destination() {
    let directory = TestDirectory::new("head-destination");
    let repository = create_initialized_project(&directory);
    let destination = directory.path().join("SampleProject.heads/payment");
    fs::create_dir(&destination).expect("conflicting destination should be created");
    fs::write(destination.join("preserve.txt"), b"owned elsewhere\n")
        .expect("conflicting content should be created");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
    assert_eq!(
        fs::read(destination.join("preserve.txt")).expect("content should be preserved"),
        b"owned elsewhere\n"
    );
    assert!(
        !head_state_lock_path(&repository).exists(),
        "destination conflict must release the state lock"
    );
}

#[test]
fn head_create_preserves_a_preexisting_branch() {
    let directory = TestDirectory::new("head-branch");
    let repository = create_initialized_project(&directory);
    let output = run_git(&repository, &["branch", "hydra/payment"]);
    assert!(output.status.success());

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
    assert!(
        !directory
            .path()
            .join("SampleProject.heads/payment")
            .exists()
    );
    let output = run_git(
        &repository,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            "refs/heads/hydra/payment",
        ],
    );
    assert!(output.status.success(), "preexisting branch must remain");
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn head_create_rejects_a_duplicate_without_altering_the_existing_head() {
    let directory = TestDirectory::new("head-duplicate");
    let repository = create_initialized_project(&directory);
    let first = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");
    assert!(
        first.status.success(),
        "first creation should succeed, stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let second = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!second.status.success());
    assert!(String::from_utf8_lossy(&second.stderr).contains("already exists"));
    assert!(
        directory
            .path()
            .join("SampleProject.heads/payment/src/app.txt")
            .is_file()
    );
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn head_create_requires_a_target_when_the_base_is_an_explicit_commit() {
    let directory = TestDirectory::new("head-commit-target");
    let repository = create_initialized_project(&directory);
    let commit = run_git(&repository, &["rev-parse", "HEAD"]);
    assert!(commit.status.success());
    let commit = String::from_utf8(commit.stdout)
        .expect("commit should be UTF-8")
        .trim()
        .to_owned();

    let output = hydra_command()
        .args(["head", "create", "payment", "--from", &commit])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--target is required"),
        "error should explain the missing integration target, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_no_head_creation_artifacts(&repository, "payment");
}

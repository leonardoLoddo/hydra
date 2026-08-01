mod common;

use std::{fs, path::Path};

use common::{
    TestDirectory, create_initialized_project, head_state_path, heads_directory, hydra_command,
    run_git,
};

fn create_head(repository: &Path, name: &str) {
    let output = hydra_command()
        .args(["head", "create", name])
        .current_dir(repository)
        .output()
        .expect("Hydra CLI should start");
    assert!(output.status.success());
}

fn commit_all(repository: &Path, message: &str) {
    assert!(run_git(repository, &["add", "."]).status.success());
    assert!(
        run_git(
            repository,
            &[
                "-c",
                "user.name=Hydra Tests",
                "-c",
                "user.email=hydra-tests@example.invalid",
                "commit",
                "--quiet",
                "-m",
                message,
            ],
        )
        .status
        .success()
    );
}

fn revision(repository: &Path, reference: &str) -> String {
    String::from_utf8(run_git(repository, &["rev-parse", reference]).stdout)
        .expect("revision should be UTF-8")
        .trim()
        .to_owned()
}

fn park_primary_worktree(repository: &Path) {
    assert!(
        run_git(repository, &["switch", "--quiet", "-c", "parking"])
            .status
            .success()
    );
    assert!(
        run_git(repository, &["config", "user.name", "Hydra Tests"])
            .status
            .success()
    );
    assert!(
        run_git(
            repository,
            &["config", "user.email", "hydra-tests@example.invalid"]
        )
        .status
        .success()
    );
}

fn configure_close(repository: &Path, program: &str, args: &[&str], remove_on_success: bool) {
    let path = repository.join(".hydra.json");
    let mut configuration: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("configuration should be readable"))
            .expect("configuration should be valid JSON");
    configuration["commands"] = serde_json::json!({
        "close": {
            "strategy": "command",
            "program": program,
            "args": args,
            "removeOnSuccess": remove_on_success,
        }
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&configuration).expect("configuration should serialize"),
    )
    .expect("configuration fixture should be written");
}

#[test]
fn custom_close_can_complete_without_removing_the_head() {
    let directory = TestDirectory::new("head-close-custom-preserve");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    configure_close(
        &repository,
        "printf",
        &[
            "%s|%s|%s|%s|%s\\n",
            "{name}",
            "{path}",
            "{headRef}",
            "{baseRef}",
            "{targetRef}",
        ],
        false,
    );
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "custom close should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be UTF-8"),
        format!(
            "payment|{}|refs/heads/hydra/payment|refs/heads/main|refs/heads/main\nClose command completed for Head payment; Head preserved\n",
            head.display()
        )
    );
    assert!(head.is_dir());
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
}

#[test]
fn custom_close_removes_the_head_after_the_command_integrates_it() {
    let directory = TestDirectory::new("head-close-custom-remove");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "feature");
    let expected = revision(&head, "HEAD");
    park_primary_worktree(&repository);
    configure_close(
        &repository,
        "git",
        &["update-ref", "{targetRef}", "{headRef}"],
        true,
    );

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "custom close should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be UTF-8"),
        "Close command completed for Head payment; Head removed\n"
    );
    assert_eq!(revision(&repository, "main"), expected);
    assert!(!head.exists());
    assert!(
        !run_git(
            &repository,
            [
                "show-ref",
                "--verify",
                "--quiet",
                "refs/heads/hydra/payment"
            ]
            .as_slice()
        )
        .status
        .success()
    );
}

#[test]
fn custom_close_reports_completed_command_when_protected_removal_fails() {
    let directory = TestDirectory::new("head-close-custom-removal-failure");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "feature");
    let head_before = revision(&head, "HEAD");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");
    configure_close(&repository, "git", &["status", "--short"], true);

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("close command completed"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("protected removal failed"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("Head was preserved"), "stderr: {stderr}");
    assert!(head.is_dir());
    assert_eq!(
        revision(&repository, "refs/heads/hydra/payment"),
        head_before
    );
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
}

#[test]
fn failing_custom_close_reports_a_target_change_and_preserves_the_head() {
    let directory = TestDirectory::new("head-close-custom-target-change");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "feature");
    let target_before = revision(&repository, "main");
    let head_before = revision(&head, "HEAD");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");
    park_primary_worktree(&repository);
    configure_close(
        &repository,
        "sh",
        &[
            "-c",
            "git update-ref \"$1\" \"$2\"; exit 7",
            "hydra-close",
            "{targetRef}",
            "{headRef}",
        ],
        true,
    );

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exited with status 7"), "stderr: {stderr}");
    assert!(
        stderr.contains(&format!(
            "target refs/heads/main changed from {target_before} to {head_before}"
        )),
        "stderr: {stderr}"
    );
    assert_eq!(revision(&repository, "main"), head_before);
    assert!(head.is_dir());
    assert_eq!(
        revision(&repository, "refs/heads/hydra/payment"),
        head_before
    );
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
}

#[test]
fn custom_close_rejects_an_unsupported_placeholder_before_starting_the_adapter() {
    let directory = TestDirectory::new("head-close-custom-placeholder");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    let target_before = revision(&repository, "main");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");
    configure_close(
        &repository,
        "git",
        &["update-ref", "{targetRef}", "{unknown}"],
        true,
    );

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported placeholder"));
    assert_eq!(revision(&repository, "main"), target_before);
    assert!(head.is_dir());
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
}

#[test]
fn custom_close_reports_when_the_configured_program_cannot_start() {
    let directory = TestDirectory::new("head-close-custom-unavailable");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");
    configure_close(
        &repository,
        "hydra-test-program-that-does-not-exist",
        &[],
        true,
    );

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("could not start close command"));
    assert!(head.is_dir());
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
}

#[test]
fn custom_close_rejects_a_dirty_head_without_suggesting_force() {
    let directory = TestDirectory::new("head-close-custom-dirty");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("uncommitted.txt"), b"work\n").expect("change should be written");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");
    configure_close(
        &repository,
        "git",
        &["update-ref", "{targetRef}", "{headRef}"],
        true,
    );

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot be closed with uncommitted changes"));
    assert!(!stderr.contains("--force"));
    assert!(head.is_dir());
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
}

#[test]
fn head_close_fast_forwards_the_target_and_removes_the_head() {
    let directory = TestDirectory::new("head-close-fast-forward");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "feature");
    let expected = revision(&head, "HEAD");
    park_primary_worktree(&repository);

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "close should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(revision(&repository, "main"), expected);
    assert!(!head.exists());
    assert!(
        !run_git(
            &repository,
            &[
                "show-ref",
                "--verify",
                "--quiet",
                "refs/heads/hydra/payment"
            ]
        )
        .status
        .success()
    );
}

#[test]
fn head_close_rejects_a_target_checked_out_in_another_worktree() {
    let directory = TestDirectory::new("head-close-open-target");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "feature");
    let target_before = revision(&repository, "main");

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("checked out"));
    assert_eq!(revision(&repository, "main"), target_before);
    assert!(head.is_dir());
}

#[test]
fn head_close_preserves_both_refs_and_the_head_on_merge_conflict() {
    let directory = TestDirectory::new("head-close-conflict");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("src/app.txt"), b"head\n").expect("Head change should be written");
    commit_all(&head, "Head change");
    fs::write(repository.join("src/app.txt"), b"target\n")
        .expect("target change should be written");
    commit_all(&repository, "target change");
    let target_before = revision(&repository, "main");
    let head_before = revision(&head, "HEAD");
    park_primary_worktree(&repository);

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("conflict"));
    assert_eq!(revision(&repository, "main"), target_before);
    assert_eq!(
        revision(&repository, "refs/heads/hydra/payment"),
        head_before
    );
    assert!(head.is_dir());
}

#[test]
fn head_close_creates_an_isolated_merge_commit_for_diverged_non_conflicting_work() {
    let directory = TestDirectory::new("head-close-merge");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "Head feature");
    let head_commit = revision(&head, "HEAD");
    fs::write(repository.join("target.txt"), b"target\n").expect("target file should be written");
    commit_all(&repository, "Target progress");
    let target_commit = revision(&repository, "main");
    park_primary_worktree(&repository);

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "non-conflicting merge should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let merged = revision(&repository, "main");
    let parents =
        String::from_utf8(run_git(&repository, &["show", "-s", "--format=%P", &merged]).stdout)
            .expect("parents should be UTF-8");
    assert_eq!(
        parents.split_ascii_whitespace().collect::<Vec<_>>(),
        [target_commit.as_str(), head_commit.as_str()]
    );
    assert!(!head.exists());
}

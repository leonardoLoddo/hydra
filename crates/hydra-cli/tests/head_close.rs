mod common;

use std::{fs, path::Path};

use common::{TestDirectory, create_initialized_project, heads_directory, hydra_command, run_git};

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

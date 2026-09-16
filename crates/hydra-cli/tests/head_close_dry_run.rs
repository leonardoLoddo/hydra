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
    assert!(run_git(repository, &["add", "--all"]).status.success());
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
    fs::write(path, serde_json::to_vec_pretty(&configuration).unwrap()).unwrap();
}

#[test]
fn close_dry_run_reports_fast_forward_without_integrating_or_removing() {
    let directory = TestDirectory::new("head-close-dry-run-json");
    let repository = create_initialized_project(&directory);
    commit_all(&repository, "configure Hydra");
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "feature");
    let target_before = revision(&repository, "main");
    let head_commit = revision(&head, "HEAD");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");

    let output = hydra_command()
        .args(["head", "close", "payment", "--dry-run", "--json"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert!(output.stdout.ends_with(b"\n"));
    assert!(!output.stdout[..output.stdout.len() - 1].contains(&b'\n'));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schemaVersion"], 1);
    assert_eq!(report["command"], "headClose");
    assert_eq!(report["name"], "payment");
    assert_eq!(report["targetRef"], "refs/heads/main");
    assert_eq!(report["targetCommit"], target_before);
    assert_eq!(report["headCommit"], head_commit);
    assert_eq!(report["strategy"]["type"], "targetWorktree");
    assert_eq!(report["integrationResult"], "fastForward");
    assert_eq!(revision(&repository, "main"), target_before);
    assert!(head.is_dir());
    assert_eq!(
        fs::read(head_state_path(&repository)).unwrap(),
        state_before
    );

    let human = hydra_command()
        .args(["head", "close", "payment", "--dry-run"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");
    assert!(human.status.success());
    let stdout = String::from_utf8(human.stdout).unwrap();
    assert!(stdout.contains("Head close plan for payment"));
    assert!(stdout.contains("Integration result: fast-forward"));
    assert!(stdout.contains("No changes made"));
    assert_eq!(revision(&repository, "main"), target_before);
    assert!(head.is_dir());
}

#[test]
fn close_dry_run_validates_cleanliness_without_mutating_the_head() {
    let directory = TestDirectory::new("head-close-dry-run-dirty");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("uncommitted.txt"), b"work\n").expect("work should be written");

    let output = hydra_command()
        .args(["head", "close", "payment", "--dry-run"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("uncommitted changes"));
    assert!(head.join("uncommitted.txt").is_file());
}

#[test]
fn close_dry_run_expands_but_does_not_execute_the_configured_adapter() {
    let directory = TestDirectory::new("head-close-dry-run-command");
    let repository = create_initialized_project(&directory);
    configure_close(
        &repository,
        "git",
        &["update-ref", "{targetRef}", "{headRef}"],
        true,
    );
    commit_all(&repository, "configure close adapter");
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").unwrap();
    commit_all(&head, "feature");
    let target_before = revision(&repository, "main");

    let output = hydra_command()
        .args(["head", "close", "payment", "--dry-run", "--json"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["strategy"]["type"], "command");
    assert_eq!(report["strategy"]["program"], "git");
    assert_eq!(report["strategy"]["args"][1], "refs/heads/main");
    assert_eq!(report["strategy"]["args"][2], "refs/heads/hydra/payment");
    assert_eq!(report["strategy"]["removeOnSuccess"], true);
    assert!(report["integrationResult"].is_null());
    assert_eq!(revision(&repository, "main"), target_before);
    assert!(head.is_dir());
}

#[test]
fn close_json_is_rejected_without_dry_run() {
    let output = hydra_command()
        .args(["head", "close", "payment", "--json"])
        .output()
        .expect("Hydra CLI should start");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--dry-run"));
}

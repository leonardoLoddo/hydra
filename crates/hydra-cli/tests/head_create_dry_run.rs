mod common;

use std::fs;

use common::{
    TestDirectory, create_initialized_project, head_state_lock_path, head_state_path,
    heads_directory, hydra_command, run_git,
};

fn commit_all(repository: &std::path::Path, message: &str) {
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

#[test]
fn create_dry_run_reports_a_json_plan_without_mutating_project_state() {
    let directory = TestDirectory::new("head-create-dry-run-json");
    let repository = create_initialized_project(&directory);
    fs::write(repository.join(".gitignore"), b".env\n").expect("ignore file should be written");
    commit_all(&repository, "ignore environment");
    fs::write(repository.join(".env"), b"local\n").expect("overlay should be written");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");

    let output = hydra_command()
        .args(["head", "create", "payment", "--dry-run", "--json"])
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
    assert_eq!(report["command"], "headCreate");
    assert_eq!(report["name"], "payment");
    assert_eq!(report["headRef"], "refs/heads/hydra/payment");
    assert_eq!(report["baseRef"], "refs/heads/main");
    assert_eq!(report["targetRef"], "refs/heads/main");
    assert_eq!(report["overlay"]["files"], 1);
    assert_eq!(report["overlay"]["bytes"], 6);
    assert!(report["trackedEntries"].as_u64().is_some());
    assert_eq!(
        fs::read(head_state_path(&repository)).unwrap(),
        state_before
    );
    assert!(!heads_directory(&repository).join("payment").exists());
    assert!(!head_state_lock_path(&repository).exists());
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
fn create_dry_run_human_output_is_explicitly_non_mutating() {
    let directory = TestDirectory::new("head-create-dry-run-human");
    let repository = create_initialized_project(&directory);

    let output = hydra_command()
        .args(["head", "create", "payment", "--dry-run"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Head creation plan for payment"));
    assert!(stdout.contains("Private branch: refs/heads/hydra/payment"));
    assert!(stdout.contains("No changes made"));
    assert!(!heads_directory(&repository).join("payment").exists());
}

#[test]
fn create_json_is_rejected_without_dry_run() {
    let output = hydra_command()
        .args(["head", "create", "payment", "--json"])
        .output()
        .expect("Hydra CLI should start");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--dry-run"));
}

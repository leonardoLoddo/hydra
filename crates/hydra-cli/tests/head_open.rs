mod common;

use std::{fs, path::Path};

use common::{
    TestDirectory, create_initialized_project, head_state_lock_path, head_state_path,
    heads_directory, hydra_command, run_git,
};

fn create_head(repository: &Path, name: &str) {
    let output = hydra_command()
        .args(["head", "create", name])
        .current_dir(repository)
        .output()
        .expect("Hydra CLI should start");
    assert!(
        output.status.success(),
        "Head creation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn configure_open(repository: &Path, program: &str, args: &[&str]) {
    let path = repository.join(".hydra.json");
    let mut configuration: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("configuration should be readable"))
            .expect("configuration should be valid JSON");
    configuration["commands"] = serde_json::json!({
        "open": {
            "program": program,
            "args": args,
        }
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&configuration).expect("configuration should serialize"),
    )
    .expect("configuration fixture should be written");
}

#[test]
fn head_open_requires_an_explicit_configured_command_without_mutation() {
    let directory = TestDirectory::new("head-open-unconfigured");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");

    let output = hydra_command()
        .args(["head", "open", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("open command is not configured"));
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn head_open_from_a_head_uses_parent_config_and_expands_placeholders() {
    let directory = TestDirectory::new("head-open-placeholders");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    configure_open(
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
    );
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");

    let output = hydra_command()
        .args(["head", "open", "payment"])
        .current_dir(&head)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "configured opener should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be UTF-8"),
        format!(
            "payment|{}|refs/heads/hydra/payment|refs/heads/main|refs/heads/main\nOpened Head payment at {}\n",
            head.display(),
            head.display()
        )
    );
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn head_open_propagates_a_non_zero_adapter_exit_without_mutation() {
    let directory = TestDirectory::new("head-open-failure");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    configure_open(
        &repository,
        "git",
        &["rev-parse", "--verify", "refs/heads/does-not-exist"],
    );
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");

    let output = hydra_command()
        .args(["head", "open", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("open command failed"));
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn head_open_rejects_a_missing_worktree_before_starting_the_adapter() {
    let directory = TestDirectory::new("head-open-missing");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    configure_open(&repository, "touch", &["adapter-was-started"]);
    let removed = run_git(
        &repository,
        &["worktree", "remove", "--force", head.to_str().unwrap()],
    );
    assert!(removed.status.success());

    let output = hydra_command()
        .args(["head", "open", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be opened safely"));
    assert!(!head.join("adapter-was-started").exists());
    assert!(!repository.join("adapter-was-started").exists());
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn head_open_starts_the_adapter_from_the_head_directory() {
    let directory = TestDirectory::new("head-open-working-directory");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    configure_open(&repository, "git", &["rev-parse", "--show-toplevel"]);

    let output = hydra_command()
        .args(["head", "open", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(
        stdout.starts_with(&format!("{}\n", head.display())),
        "adapter should observe the Head as its working directory, got: {stdout:?}"
    );
}

#[test]
fn head_open_rejects_an_unsupported_placeholder_before_starting_the_adapter() {
    let directory = TestDirectory::new("head-open-unknown-placeholder");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    configure_open(&repository, "touch", &["{unknown}"]);

    let output = hydra_command()
        .args(["head", "open", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported placeholder"));
    assert!(!head.join("{unknown}").exists());
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn head_open_rejects_a_worktree_checked_out_on_a_different_branch() {
    let directory = TestDirectory::new("head-open-branch-mismatch");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    configure_open(&repository, "touch", &["adapter-was-started"]);
    let switched = run_git(&head, &["switch", "--quiet", "-c", "other-work"]);
    assert!(switched.status.success());

    let output = hydra_command()
        .args(["head", "open", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("branch does not match"));
    assert!(!head.join("adapter-was-started").exists());
    assert!(!head_state_lock_path(&repository).exists());
}

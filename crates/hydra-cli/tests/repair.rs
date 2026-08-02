mod common;

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Output, Stdio},
};

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

fn run_repair(repository: &Path, input: &[u8]) -> Output {
    let mut child = hydra_command()
        .arg("repair")
        .current_dir(repository)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Hydra CLI should start");
    child
        .stdin
        .take()
        .expect("repair stdin should be piped")
        .write_all(input)
        .expect("repair input should be written");
    child.wait_with_output().expect("repair should finish")
}

fn head_is_recorded(repository: &Path, name: &str) -> bool {
    let state: serde_json::Value = serde_json::from_slice(
        &fs::read(head_state_path(repository)).expect("state should be readable"),
    )
    .expect("state should be valid JSON");
    state["heads"].get(name).is_some()
}

fn branch_exists(repository: &Path, name: &str) -> bool {
    run_git(
        repository,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/hydra/{name}"),
        ],
    )
    .status
    .success()
}

fn recovery_manifest_path(repository: &Path, name: &str) -> PathBuf {
    let head = heads_directory(repository).join(name);
    let output = run_git(
        &head,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "hydra-head.json",
        ],
    );
    assert!(output.status.success());
    PathBuf::from(
        String::from_utf8(output.stdout)
            .expect("recovery manifest fixture path should be UTF-8")
            .trim_end(),
    )
}

#[test]
fn repair_requires_confirmation_before_rebuilding_a_missing_inventory() {
    let directory = TestDirectory::new("repair-missing-inventory-declined");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    let state_path = head_state_path(&repository);
    fs::remove_file(&state_path).expect("disposable inventory should be removed");

    let output = run_repair(&repository, b"\n");

    assert!(
        output.status.success(),
        "repair planning should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains(&format!(
        "Missing Hydra inventory: {}",
        state_path.display()
    )));
    assert!(stdout.contains("Recoverable Head: payment"));
    assert!(stdout.contains("Rebuild the missing inventory with 1 recovered Head? [y/N] "));
    assert!(stdout.ends_with("No repairs applied.\n"));
    assert!(
        !state_path.exists(),
        "declining must preserve the missing state"
    );
    assert!(head.is_dir());
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn confirmed_repair_rebuilds_a_missing_inventory_from_recovery_manifests() {
    let directory = TestDirectory::new("repair-missing-inventory-confirmed");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    let state_path = head_state_path(&repository);
    let state_before = fs::read(&state_path).expect("inventory should be readable");
    fs::remove_file(&state_path).expect("disposable inventory should be removed");

    let output = run_repair(&repository, b"yes\n");

    assert!(
        output.status.success(),
        "confirmed inventory recovery should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .expect("stdout should be UTF-8")
            .ends_with("Rebuilt the missing inventory with 1 recovered Head.\n")
    );
    assert_eq!(
        fs::read(&state_path).expect("inventory should be rebuilt"),
        state_before,
        "recovery must preserve the original Head intent"
    );
    assert!(head_is_recorded(&repository, "payment"));
    assert!(head.is_dir());
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn repair_does_not_rebuild_when_a_head_has_no_recovery_manifest() {
    let directory = TestDirectory::new("repair-missing-recovery-manifest");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    let state_path = head_state_path(&repository);
    fs::remove_file(recovery_manifest_path(&repository, "payment"))
        .expect("disposable recovery manifest should be removed");
    fs::remove_file(&state_path).expect("disposable inventory should be removed");

    let output = run_repair(&repository, b"yes\n");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("Untracked Hydra worktree: payment"));
    assert!(stdout.ends_with("No automatic repairs available; manual recovery required.\n"));
    assert!(!state_path.exists());
    assert!(head.is_dir());
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn repair_does_not_replace_a_malformed_inventory_from_recovery_manifests() {
    let directory = TestDirectory::new("repair-malformed-inventory");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let state_path = head_state_path(&repository);
    let malformed = b"not valid inventory\n";
    fs::write(&state_path, malformed).expect("malformed fixture should be written");

    let output = run_repair(&repository, b"yes\n");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("is invalid"));
    assert_eq!(
        fs::read(&state_path).expect("malformed inventory should remain"),
        malformed
    );
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn repair_reports_a_consistent_project_without_mutating_inventory() {
    let directory = TestDirectory::new("repair-consistent");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");

    let output = run_repair(&repository, b"");

    assert!(
        output.status.success(),
        "repair should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be UTF-8"),
        "Hydra state is consistent.\n"
    );
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn repair_requires_confirmation_before_removing_stale_inventory() {
    let directory = TestDirectory::new("repair-stale-declined");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    let removed = run_git(
        &repository,
        &["worktree", "remove", "--force", head.to_str().unwrap()],
    );
    assert!(removed.status.success());

    let output = run_repair(&repository, b"\n");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("Stale inventory: payment"));
    assert!(stdout.contains("Remove 1 stale inventory entry while preserving its branch? [y/N] "));
    assert!(stdout.ends_with("No repairs applied.\n"));
    assert!(head_is_recorded(&repository, "payment"));
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn confirmed_repair_removes_only_stale_inventory_and_preserves_the_branch() {
    let directory = TestDirectory::new("repair-stale-confirmed");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    let removed = run_git(
        &repository,
        &["worktree", "remove", "--force", head.to_str().unwrap()],
    );
    assert!(removed.status.success());

    let output = run_repair(&repository, b"yes\n");

    assert!(
        output.status.success(),
        "confirmed repair should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("Stale inventory: payment"));
    assert!(stdout.ends_with("Removed 1 stale inventory entry.\n"));
    assert!(!head_is_recorded(&repository, "payment"));
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn repair_reports_an_untracked_hydra_worktree_without_guessing_metadata() {
    let directory = TestDirectory::new("repair-untracked-worktree");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    let state_path = head_state_path(&repository);
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).expect("state should be readable"))
            .expect("state should be valid JSON");
    state["heads"]
        .as_object_mut()
        .expect("heads should be an object")
        .remove("payment");
    fs::write(
        &state_path,
        serde_json::to_vec_pretty(&state).expect("state should serialize"),
    )
    .expect("state fixture should be written");
    let state_before = fs::read(&state_path).expect("state should be readable");

    let output = run_repair(&repository, b"");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains(&format!(
        "Untracked Hydra worktree: payment at {}",
        head.display()
    )));
    assert!(stdout.contains("manual recovery required"));
    assert_eq!(
        fs::read(&state_path).expect("state should remain"),
        state_before
    );
    assert!(head.is_dir());
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn repair_reports_a_moved_worktree_without_relocating_it_silently() {
    let directory = TestDirectory::new("repair-moved-worktree");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let original = heads_directory(&repository).join("payment");
    let moved = directory.path().join("moved-payment");
    let moved_output = run_git(
        &repository,
        &[
            "worktree",
            "move",
            original.to_str().unwrap(),
            moved.to_str().unwrap(),
        ],
    );
    assert!(moved_output.status.success());
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");

    let output = run_repair(&repository, b"");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let registered_moved =
        fs::canonicalize(&moved).expect("moved worktree path should be resolvable");
    assert!(
        stdout.contains(&format!(
            "Moved Head worktree: payment is registered at {}",
            registered_moved.display()
        )),
        "repair should identify the relocated worktree, got: {stdout:?}"
    );
    assert!(stdout.contains("Move 1 relocated Head worktree back to its managed path? [y/N] "));
    assert!(stdout.ends_with("No repairs applied.\n"));
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain"),
        state_before
    );
    assert!(moved.is_dir());
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn confirmed_repair_moves_a_relocated_worktree_back_to_its_managed_path() {
    let directory = TestDirectory::new("repair-moved-confirmed");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let managed = heads_directory(&repository).join("payment");
    let moved = directory.path().join("moved-payment");
    let moved_output = run_git(
        &repository,
        &[
            "worktree",
            "move",
            managed.to_str().unwrap(),
            moved.to_str().unwrap(),
        ],
    );
    assert!(moved_output.status.success());

    let output = run_repair(&repository, b"yes\n");

    assert!(
        output.status.success(),
        "confirmed relocation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .expect("stdout should be UTF-8")
            .ends_with("Restored 1 Head worktree to its managed path.\n")
    );
    assert!(managed.is_dir());
    assert!(!moved.exists());
    assert!(head_is_recorded(&repository, "payment"));
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn repair_detects_a_registered_worktree_whose_directory_was_deleted() {
    let directory = TestDirectory::new("repair-deleted-directory");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::remove_dir_all(&head).expect("disposable Head directory should be removed");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");

    let output = run_repair(&repository, b"");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains(&format!(
        "Missing registered worktree: payment at {}",
        head.display()
    )));
    assert!(stdout.contains("manual recovery required"));
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain"),
        state_before
    );
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn repair_rejects_an_inventory_path_outside_the_managed_directory() {
    let directory = TestDirectory::new("repair-unsafe-path");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let state_path = head_state_path(&repository);
    let outside = directory.path().join("outside");
    fs::create_dir(&outside).expect("outside fixture should be created");
    fs::write(outside.join("preserve.txt"), b"preserve\n")
        .expect("outside fixture should be written");
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).expect("state should be readable"))
            .expect("state should be valid JSON");
    state["heads"]["payment"]["worktreePath"] = outside
        .to_str()
        .expect("fixture path should be UTF-8")
        .into();
    fs::write(
        &state_path,
        serde_json::to_vec_pretty(&state).expect("state should serialize"),
    )
    .expect("unsafe state fixture should be written");
    let state_before = fs::read(&state_path).expect("state should be readable");

    let output = run_repair(&repository, b"yes\n");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsafe"));
    assert_eq!(
        fs::read(outside.join("preserve.txt")).expect("outside content should remain"),
        b"preserve\n"
    );
    assert_eq!(
        fs::read(&state_path).expect("state should remain"),
        state_before
    );
    assert!(!head_state_lock_path(&repository).exists());
}

mod common;

use std::{
    fs::{self, OpenOptions},
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

fn write_pending_creation(repository: &Path, name: &str) -> PathBuf {
    let head_ref = format!("refs/heads/hydra/{name}");
    let head_path = heads_directory(repository).join(name);
    let base_commit = run_git(repository, &["rev-parse", "HEAD"]);
    assert!(base_commit.status.success());
    let base_commit = String::from_utf8(base_commit.stdout)
        .expect("base commit should be UTF-8")
        .trim()
        .to_owned();
    let path = heads_directory(repository)
        .join(".hydra")
        .join(format!("pending-{name}.json"));
    let record = serde_json::json!({
        "version": 1,
        "name": name,
        "intent": {
            "worktreePath": head_path,
            "headRef": head_ref,
            "baseRef": "refs/heads/main",
            "baseCommit": base_commit,
            "targetRef": "refs/heads/main"
        }
    });
    let mut contents = serde_json::to_vec_pretty(&record).expect("pending record should serialize");
    contents.push(b'\n');
    fs::write(&path, contents).expect("pending creation fixture should be written");
    path
}

fn remove_head_from_inventory(repository: &Path, name: &str) -> (Vec<u8>, Vec<u8>) {
    let state_path = head_state_path(repository);
    let original = fs::read(&state_path).expect("inventory should be readable");
    let mut state: serde_json::Value =
        serde_json::from_slice(&original).expect("inventory should be valid JSON");
    state["heads"]
        .as_object_mut()
        .expect("heads should be an object")
        .remove(name)
        .expect("Head should be recorded before the crash fixture");
    let mut interrupted = serde_json::to_vec_pretty(&state).expect("inventory should serialize");
    interrupted.push(b'\n');
    fs::write(&state_path, &interrupted).expect("interrupted inventory should be written");
    (original, interrupted)
}

#[test]
fn repair_requires_confirmation_before_adopting_a_manifest_backed_head() {
    let directory = TestDirectory::new("repair-manifest-head-declined");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "existing");
    create_head(&repository, "payment");
    let (_, interrupted_inventory) = remove_head_from_inventory(&repository, "payment");

    let output = run_repair(&repository, b"\n");

    assert!(
        output.status.success(),
        "repair planning should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("Recoverable untracked Head: payment"));
    assert!(stdout.contains("Add 1 recovered Head to the inventory? [y/N] "));
    assert!(stdout.ends_with("No repairs applied.\n"));
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("inventory should remain readable"),
        interrupted_inventory
    );
    assert!(head_is_recorded(&repository, "existing"));
    assert!(!head_is_recorded(&repository, "payment"));
    assert!(heads_directory(&repository).join("payment").is_dir());
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn repair_requires_confirmation_before_cleaning_a_pre_worktree_creation() {
    let directory = TestDirectory::new("repair-pending-creation-declined");
    let repository = create_initialized_project(&directory);
    let journal = write_pending_creation(&repository, "payment");
    let output = run_git(&repository, &["branch", "hydra/payment", "HEAD"]);
    assert!(output.status.success());

    let output = run_repair(&repository, b"\n");

    assert!(
        output.status.success(),
        "repair planning should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("Incomplete Head creation: payment"));
    assert!(stdout.contains("Clean up 1 incomplete Head creation? [y/N] "));
    assert!(stdout.ends_with("No repairs applied.\n"));
    assert!(journal.is_file());
    assert!(branch_exists(&repository, "payment"));
    assert!(!heads_directory(&repository).join("payment").exists());
    assert!(!head_is_recorded(&repository, "payment"));
}

#[test]
fn confirmed_repair_cleans_a_pre_worktree_creation_with_an_unchanged_branch() {
    let directory = TestDirectory::new("repair-pending-creation-confirmed");
    let repository = create_initialized_project(&directory);
    let journal = write_pending_creation(&repository, "payment");
    let output = run_git(&repository, &["branch", "hydra/payment", "HEAD"]);
    assert!(output.status.success());

    let output = run_repair(&repository, b"yes\n");

    assert!(
        output.status.success(),
        "confirmed cleanup should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .expect("stdout should be UTF-8")
            .ends_with("Cleaned up 1 incomplete Head creation.\n")
    );
    assert!(!journal.exists());
    assert!(!branch_exists(&repository, "payment"));
    assert!(!heads_directory(&repository).join("payment").exists());
    assert!(!head_is_recorded(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn pending_creation_recovery_rechecks_the_branch_commit_after_planning() {
    let directory = TestDirectory::new("repair-pending-creation-race");
    let repository = create_initialized_project(&directory);
    let journal = write_pending_creation(&repository, "payment");
    let output = run_git(&repository, &["branch", "hydra/payment", "HEAD"]);
    assert!(output.status.success());
    let plan = hydra_core::plan_repairs(&repository).expect("repair planning should succeed");
    assert_eq!(plan.recoverable_pending_creations, ["payment"]);

    fs::write(repository.join("after-plan.txt"), b"new commit\n")
        .expect("new tracked file should be written");
    let output = run_git(&repository, &["add", "after-plan.txt"]);
    assert!(output.status.success());
    let output = run_git(
        &repository,
        &[
            "-c",
            "user.name=Hydra Tests",
            "-c",
            "user.email=hydra-tests@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "advance after planning",
        ],
    );
    assert!(output.status.success());
    let output = run_git(&repository, &["branch", "--force", "hydra/payment", "HEAD"]);
    assert!(output.status.success());

    let result = hydra_core::apply_pending_creation_recovery(
        &repository,
        &plan.recoverable_pending_creations,
    )
    .expect("changed recovery candidate should be preserved");

    assert!(result.cleaned_creations.is_empty());
    assert!(journal.is_file());
    assert!(branch_exists(&repository, "payment"));
    assert!(!heads_directory(&repository).join("payment").exists());
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn pending_journal_cleanup_preserves_an_already_recorded_head() {
    let directory = TestDirectory::new("repair-committed-pending-journal");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let journal = write_pending_creation(&repository, "payment");
    let head = heads_directory(&repository).join("payment");

    let output = run_repair(&repository, b"yes\n");

    assert!(
        output.status.success(),
        "journal cleanup should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!journal.exists());
    assert!(head.is_dir());
    assert!(head_is_recorded(&repository, "payment"));
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn confirmed_repair_adopts_a_manifest_backed_head_without_changing_existing_entries() {
    let directory = TestDirectory::new("repair-manifest-head-confirmed");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "existing");
    create_head(&repository, "payment");
    let (complete_inventory, _) = remove_head_from_inventory(&repository, "payment");

    let output = run_repair(&repository, b"yes\n");

    assert!(
        output.status.success(),
        "confirmed Head adoption should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .expect("stdout should be UTF-8")
            .ends_with("Added 1 recovered Head to the inventory.\n")
    );
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("inventory should remain readable"),
        complete_inventory,
        "adoption should restore exact metadata and preserve existing entries"
    );
    assert!(head_is_recorded(&repository, "existing"));
    assert!(head_is_recorded(&repository, "payment"));
    assert!(heads_directory(&repository).join("payment").is_dir());
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn untracked_head_recovery_rechecks_the_manifest_after_planning() {
    let directory = TestDirectory::new("repair-manifest-head-race");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let (_, interrupted_inventory) = remove_head_from_inventory(&repository, "payment");
    let plan = hydra_core::plan_repairs(&repository).expect("repair planning should succeed");
    assert_eq!(plan.recoverable_untracked_heads, ["payment"]);
    fs::remove_file(recovery_manifest_path(&repository, "payment"))
        .expect("fixture should remove the recovery manifest after planning");

    let result =
        hydra_core::apply_untracked_head_recovery(&repository, &plan.recoverable_untracked_heads)
            .expect("changed recovery state should be skipped safely");

    assert!(result.recovered_heads.is_empty());
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("inventory should remain readable"),
        interrupted_inventory
    );
    assert!(!head_is_recorded(&repository, "payment"));
    assert!(heads_directory(&repository).join("payment").is_dir());
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn repair_does_not_adopt_a_head_with_an_inconsistent_recovery_manifest() {
    let directory = TestDirectory::new("repair-inconsistent-head-manifest");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let (_, interrupted_inventory) = remove_head_from_inventory(&repository, "payment");
    let manifest_path = recovery_manifest_path(&repository, "payment");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("manifest should be readable"))
            .expect("manifest should be valid JSON");
    manifest["name"] = serde_json::json!("other");
    let mut manifest_bytes =
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize");
    manifest_bytes.push(b'\n');
    fs::write(&manifest_path, manifest_bytes).expect("fixture manifest should be replaced");

    let output = run_repair(&repository, b"yes\n");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("Untracked Hydra worktree: payment"));
    assert!(stdout.ends_with("No automatic repairs available; manual recovery required.\n"));
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("inventory should remain readable"),
        interrupted_inventory
    );
    assert!(!head_is_recorded(&repository, "payment"));
    assert!(heads_directory(&repository).join("payment").is_dir());
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn repair_requires_confirmation_before_removing_an_abandoned_current_state_lock() {
    let directory = TestDirectory::new("repair-abandoned-state-lock-declined");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let lock_path = head_state_lock_path(&repository);
    let inventory_before = fs::read(head_state_path(&repository)).expect("inventory should exist");
    fs::write(&lock_path, b"{\n  \"version\": 1\n}\n")
        .expect("current abandoned lock fixture should be written");

    let output = run_repair(&repository, b"\n");

    assert!(
        output.status.success(),
        "repair planning should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains(&format!(
        "Abandoned Hydra state lock: {}",
        lock_path.display()
    )));
    assert!(stdout.contains("Remove the abandoned Hydra state lock? [y/N] "));
    assert!(stdout.ends_with("No repairs applied.\n"));
    assert!(lock_path.is_file(), "declining must preserve the lock");
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("inventory should remain readable"),
        inventory_before
    );
    assert!(heads_directory(&repository).join("payment").is_dir());
    assert!(branch_exists(&repository, "payment"));
}

#[test]
fn confirmed_repair_removes_an_abandoned_current_state_lock_only() {
    let directory = TestDirectory::new("repair-abandoned-state-lock-confirmed");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let lock_path = head_state_lock_path(&repository);
    let inventory_before = fs::read(head_state_path(&repository)).expect("inventory should exist");
    fs::write(&lock_path, b"{\n  \"version\": 1\n}\n")
        .expect("current abandoned lock fixture should be written");

    let output = run_repair(&repository, b"yes\n");

    assert!(
        output.status.success(),
        "confirmed lock recovery should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .expect("stdout should be UTF-8")
            .ends_with("Removed the abandoned Hydra state lock.\n")
    );
    assert!(
        !lock_path.exists(),
        "confirmed repair should remove the lock"
    );
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("inventory should remain readable"),
        inventory_before
    );
    assert!(heads_directory(&repository).join("payment").is_dir());
    assert!(branch_exists(&repository, "payment"));
}

#[test]
fn repair_preserves_an_active_current_state_lock() {
    let directory = TestDirectory::new("repair-active-state-lock");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let lock_path = head_state_lock_path(&repository);
    fs::write(&lock_path, b"{\n  \"version\": 1\n}\n")
        .expect("current active lock fixture should be written");
    let guard_path = heads_directory(&repository).join(".hydra/directory.json");
    let guard = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&guard_path)
        .expect("state guard should open");
    guard.lock().expect("fixture should own the state guard");

    let output = run_repair(&repository, b"yes\n");

    guard
        .unlock()
        .expect("fixture should release the state guard");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains(&format!("Active Hydra state lock: {}", lock_path.display())));
    assert!(stdout.ends_with("No automatic repairs available; manual recovery required.\n"));
    assert!(lock_path.is_file(), "active lock must remain untouched");
    assert!(heads_directory(&repository).join("payment").is_dir());
    assert!(branch_exists(&repository, "payment"));
}

#[test]
fn state_lock_recovery_rechecks_process_ownership_after_planning() {
    let directory = TestDirectory::new("repair-state-lock-race");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let lock_path = head_state_lock_path(&repository);
    fs::write(&lock_path, b"{\n  \"version\": 1\n}\n")
        .expect("current abandoned lock fixture should be written");
    let plan = hydra_core::plan_repairs(&repository).expect("repair planning should succeed");
    assert_eq!(
        plan.abandoned_state_lock.as_deref(),
        Some(lock_path.as_path())
    );

    let guard_path = heads_directory(&repository).join(".hydra/directory.json");
    let guard = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&guard_path)
        .expect("state guard should open");
    guard.lock().expect("fixture should own the state guard");
    let recovery = hydra_core::apply_abandoned_state_lock_recovery(&repository);
    guard
        .unlock()
        .expect("fixture should release the state guard");

    assert!(matches!(
        recovery,
        Err(hydra_core::HeadError::StateLockExists(path)) if path == lock_path
    ));
    assert!(
        lock_path.is_file(),
        "a newly active lock must remain untouched"
    );
    assert!(heads_directory(&repository).join("payment").is_dir());
    assert!(branch_exists(&repository, "payment"));
}

#[test]
fn repair_rejects_a_malformed_state_lock_without_mutation() {
    let directory = TestDirectory::new("repair-malformed-state-lock");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let lock_path = head_state_lock_path(&repository);
    fs::write(&lock_path, b"").expect("malformed lock fixture should be written");

    let output = run_repair(&repository, b"yes\n");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("state lock"));
    assert!(lock_path.is_file(), "malformed lock must remain untouched");
    assert_eq!(
        fs::read(&lock_path).expect("malformed lock should remain readable"),
        b""
    );
    assert!(heads_directory(&repository).join("payment").is_dir());
    assert!(branch_exists(&repository, "payment"));
}

#[test]
fn repair_rejects_an_unsupported_state_lock_version_without_mutation() {
    let directory = TestDirectory::new("repair-unsupported-state-lock");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let lock_path = head_state_lock_path(&repository);
    let unsupported = b"{\n  \"version\": 2\n}\n";
    fs::write(&lock_path, unsupported).expect("unsupported lock fixture should be written");

    let output = run_repair(&repository, b"yes\n");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("lock version 2"));
    assert_eq!(
        fs::read(&lock_path).expect("unsupported lock should remain readable"),
        unsupported
    );
    assert!(heads_directory(&repository).join("payment").is_dir());
    assert!(branch_exists(&repository, "payment"));
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
    fs::remove_file(recovery_manifest_path(&repository, "payment"))
        .expect("fixture should remove the exact recovery metadata");
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

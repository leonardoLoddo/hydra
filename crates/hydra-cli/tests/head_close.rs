mod common;

use std::{
    fs,
    path::Path,
    process::Stdio,
    thread,
    time::{Duration, Instant},
};

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
    assert!(
        output.status.success(),
        "Head creation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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

fn display_path_for_git(path: &Path) -> String {
    let canonical =
        common::canonical_path(path).expect("repository path should be canonicalizable");
    canonical.display().to_string()
}

fn park_primary_worktree(repository: &Path) {
    assert!(
        run_git(repository, &["switch", "--quiet", "-c", "parking"])
            .status
            .success()
    );
    configure_git_identity(repository);
}

fn configure_git_identity(repository: &Path) {
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

fn wait_for_merge(repository: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if run_git(repository, &["rev-parse", "--verify", "MERGE_HEAD"])
            .status
            .success()
        {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("Git merge state should become visible");
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
fn custom_close_from_a_head_reports_the_parent_without_running_the_adapter() {
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
        .current_dir(&head)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty(), "adapter must not run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("head close must be run from the parent project worktree"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains(&display_path_for_git(&repository)),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("No changes were made"), "stderr: {stderr}");
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
fn removing_custom_close_from_a_head_preserves_target_and_head() {
    let directory = TestDirectory::new("head-close-custom-from-head");
    let repository = create_initialized_project(&directory);
    configure_close(
        &repository,
        "git",
        &["update-ref", "{targetRef}", "{headRef}"],
        true,
    );
    commit_all(&repository, "configure Hydra close");
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "feature");
    let target_before = revision(&repository, "main");
    let head_before = revision(&head, "HEAD");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");
    park_primary_worktree(&repository);

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&head)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("head close must be run from the parent project worktree"),
        "stderr: {stderr}"
    );
    assert_eq!(revision(&repository, "main"), target_before);
    assert_eq!(revision(&head, "HEAD"), head_before);
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
    assert!(head.exists());
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
fn head_close_requires_the_target_branch_in_the_parent_worktree() {
    let directory = TestDirectory::new("head-close-fast-forward");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "feature");
    let target_before = revision(&repository, "main");
    let head_before = revision(&head, "HEAD");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");
    park_primary_worktree(&repository);
    let canonical_repository = display_path_for_git(&repository);

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("target refs/heads/main must be checked out"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains(&canonical_repository), "stderr: {stderr}");
    assert!(
        stderr.contains("current branch is refs/heads/parking"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("No changes were made"), "stderr: {stderr}");
    assert_eq!(revision(&repository, "main"), target_before);
    assert_eq!(revision(&head, "HEAD"), head_before);
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
    assert!(head.exists());
}

#[test]
fn head_close_fast_forwards_a_clean_checked_out_target_worktree() {
    let directory = TestDirectory::new("head-close-open-target");
    let repository = create_initialized_project(&directory);
    commit_all(&repository, "configure Hydra");
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "feature");
    let expected = revision(&head, "HEAD");
    let canonical_repository = display_path_for_git(&repository);

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
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Updating "), "stdout: {stdout}");
    assert!(stdout.contains("Fast-forward"), "stdout: {stdout}");
    assert!(
        stdout.contains(&format!(
            "Integration strategy: target worktree {canonical_repository}"
        )),
        "expected target path {canonical_repository:?}, stdout: {stdout}"
    );
    assert!(
        stdout.contains("Integration result: fast-forward"),
        "stdout: {stdout}"
    );
    assert_eq!(revision(&repository, "main"), expected);
    assert_eq!(
        fs::read(repository.join("feature.txt")).expect("merged file should be readable"),
        b"feature\n"
    );
    assert!(
        run_git(&repository, &["status", "--porcelain"])
            .stdout
            .is_empty()
    );
    assert!(!head.exists());
}

#[test]
fn head_close_from_a_head_reports_the_parent_and_preserves_everything() {
    let directory = TestDirectory::new("head-close-requires-parent");
    let repository = create_initialized_project(&directory);
    commit_all(&repository, "configure Hydra");
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "feature");
    let target_before = revision(&repository, "main");
    let head_before = revision(&head, "HEAD");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");
    let canonical_repository = display_path_for_git(&repository);

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&head)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("head close must be run from the parent project worktree"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains(&canonical_repository), "stderr: {stderr}");
    assert!(stderr.contains("No changes were made"), "stderr: {stderr}");
    assert_eq!(revision(&repository, "main"), target_before);
    assert_eq!(revision(&head, "HEAD"), head_before);
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
    assert!(head.is_dir());
}

#[test]
fn head_close_reports_when_the_head_is_already_integrated() {
    let directory = TestDirectory::new("head-close-already-integrated");
    let repository = create_initialized_project(&directory);
    commit_all(&repository, "configure Hydra");
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Integration strategy: target worktree"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Integration result: already integrated"),
        "stdout: {stdout}"
    );
    assert!(!head.exists());
}

#[test]
fn head_close_reports_dirty_checked_out_target_without_mutation() {
    let directory = TestDirectory::new("head-close-dirty-target");
    let repository = create_initialized_project(&directory);
    commit_all(&repository, "configure Hydra");
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "feature");
    let target_before = revision(&repository, "main");
    let head_before = revision(&head, "HEAD");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");
    fs::write(repository.join("src/app.txt"), b"ongoing\n")
        .expect("tracked target change should be written");
    fs::write(repository.join("notes.txt"), b"untracked\n")
        .expect("untracked target change should be written");

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("1 modified, 0 added, 0 deleted, 1 untracked"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("target and Head were preserved"));
    assert_eq!(revision(&repository, "main"), target_before);
    assert_eq!(revision(&head, "HEAD"), head_before);
    assert_eq!(
        fs::read(repository.join("src/app.txt")).expect("target file should remain readable"),
        b"ongoing\n"
    );
    assert_eq!(
        fs::read(repository.join("notes.txt")).expect("untracked file should remain readable"),
        b"untracked\n"
    );
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
    assert!(head.is_dir());
}

#[test]
fn head_close_rejects_a_clean_target_with_a_git_operation_in_progress() {
    let directory = TestDirectory::new("head-close-target-operation");
    let repository = create_initialized_project(&directory);
    commit_all(&repository, "configure Hydra");
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "feature");
    let target_before = revision(&repository, "main");
    let head_before = revision(&head, "HEAD");
    let merge_head_path = String::from_utf8(
        run_git(
            &repository,
            &[
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                "MERGE_HEAD",
            ],
        )
        .stdout,
    )
    .expect("MERGE_HEAD path should be UTF-8");
    fs::write(merge_head_path.trim(), format!("{head_before}\n"))
        .expect("merge operation marker should be written");
    assert!(
        run_git(&repository, &["status", "--porcelain"])
            .stdout
            .is_empty()
    );

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("merge operation in progress"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("target and Head were preserved"));
    assert_eq!(revision(&repository, "main"), target_before);
    assert_eq!(revision(&head, "HEAD"), head_before);
    assert!(head.is_dir());
}

#[test]
fn head_close_waits_for_a_conflicted_git_merge_and_finishes_after_commit() {
    let directory = TestDirectory::new("head-close-conflict-resolved");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("src/app.txt"), b"head\n").expect("Head change should be written");
    commit_all(&head, "Head change");
    fs::write(repository.join("src/app.txt"), b"target\n")
        .expect("target change should be written");
    commit_all(&repository, "target change");
    let head_before = revision(&head, "HEAD");
    configure_git_identity(&repository);

    let child = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Hydra CLI should start");
    wait_for_merge(&repository);

    fs::write(repository.join("src/app.txt"), b"resolved\n")
        .expect("resolved file should be written");
    assert!(
        run_git(&repository, &["add", "src/app.txt"])
            .status
            .success()
    );
    assert!(
        run_git(
            &repository,
            &["-c", "core.editor=true", "merge", "--continue"]
        )
        .status
        .success()
    );

    let output = child
        .wait_with_output()
        .expect("Hydra close should finish after the merge commit");

    assert!(
        output.status.success(),
        "close should resume after commit, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CONFLICT (content)"), "stdout: {stdout}");
    assert!(
        stdout.contains("Automatic merge failed"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("Closed Head payment"), "stdout: {stdout}");
    assert!(
        run_git(
            &repository,
            &["merge-base", "--is-ancestor", &head_before, "main"]
        )
        .status
        .success()
    );
    assert_eq!(
        fs::read(repository.join("src/app.txt")).expect("resolved file should be readable"),
        b"resolved\n"
    );
    assert!(
        run_git(&repository, &["status", "--porcelain"])
            .stdout
            .is_empty()
    );
    assert!(!head.exists());
}

#[test]
fn head_close_finishes_as_aborted_when_the_user_aborts_the_git_merge() {
    let directory = TestDirectory::new("head-close-conflict-aborted");
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

    let child = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Hydra CLI should start");
    wait_for_merge(&repository);
    assert!(run_git(&repository, &["merge", "--abort"]).status.success());

    let output = child
        .wait_with_output()
        .expect("Hydra close should finish after merge abort");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("close was aborted through Git"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("Head was preserved"), "stderr: {stderr}");
    assert_eq!(revision(&repository, "main"), target_before);
    assert_eq!(revision(&head, "HEAD"), head_before);
    assert_eq!(
        fs::read(repository.join("src/app.txt")).expect("target file should be restored"),
        b"target\n"
    );
    assert!(head.is_dir());
}

#[test]
fn head_close_merges_into_a_clean_checked_out_target_worktree() {
    let directory = TestDirectory::new("head-close-merge");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "Head feature");
    let head_commit = revision(&head, "HEAD");
    fs::write(repository.join("target.txt"), b"target\n").expect("target file should be written");
    commit_all(&repository, "Target progress");
    configure_git_identity(&repository);
    let target_commit = revision(&repository, "main");
    let canonical_repository = display_path_for_git(&repository);

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
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&format!(
            "Integration strategy: target worktree {canonical_repository}"
        )),
        "expected target path {canonical_repository:?}, stdout: {stdout}"
    );
    assert!(
        stdout.contains("Integration result: merge commit"),
        "stdout: {stdout}"
    );
    let merged = revision(&repository, "main");
    let parents =
        String::from_utf8(run_git(&repository, &["show", "-s", "--format=%P", &merged]).stdout)
            .expect("parents should be UTF-8");
    assert_eq!(
        parents.split_ascii_whitespace().collect::<Vec<_>>(),
        [target_commit.as_str(), head_commit.as_str()]
    );
    assert_eq!(
        fs::read(repository.join("feature.txt")).expect("Head file should be materialized"),
        b"feature\n"
    );
    assert!(
        run_git(&repository, &["status", "--porcelain"])
            .stdout
            .is_empty()
    );
    assert!(!head.exists());
}

#[test]
fn head_close_does_not_create_a_merge_when_target_is_not_checked_out() {
    let directory = TestDirectory::new("head-close-target-not-checked-out");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature should be written");
    commit_all(&head, "Head feature");
    let head_commit = revision(&head, "HEAD");
    fs::write(repository.join("target.txt"), b"target\n").expect("target file should be written");
    commit_all(&repository, "Target progress");
    let target_commit = revision(&repository, "main");
    let state_before = fs::read(head_state_path(&repository)).expect("state should be readable");
    park_primary_worktree(&repository);

    let output = hydra_command()
        .args(["head", "close", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("target refs/heads/main must be checked out"),
        "stderr: {stderr}"
    );
    assert_eq!(revision(&repository, "main"), target_commit);
    assert_eq!(revision(&head, "HEAD"), head_commit);
    assert_eq!(
        fs::read(head_state_path(&repository)).expect("state should remain readable"),
        state_before
    );
    assert!(head.exists());
}

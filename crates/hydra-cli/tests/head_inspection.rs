mod common;

use std::fs;

use common::{
    TestDirectory, create_initialized_project, head_state_lock_path, head_state_path,
    heads_directory, hydra_command, run_git,
};

fn create_head(repository: &std::path::Path, name: &str) {
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

fn commit(repository: &std::path::Path, message: &str) {
    let output = run_git(repository, &["add", "."]);
    assert!(output.status.success());
    let output = run_git(
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
    );
    assert!(
        output.status.success(),
        "fixture commit should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn revision(repository: &std::path::Path, reference: &str) -> String {
    let output = run_git(repository, &["rev-parse", reference]);
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("revision should be UTF-8")
        .trim()
        .to_owned()
}

#[test]
fn project_status_and_head_list_report_local_heads_in_name_order_without_mutation() {
    let directory = TestDirectory::new("head-list");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    create_head(&repository, "auth");
    let state_path = head_state_path(&repository);
    let state_before = fs::read(&state_path).expect("state should be readable");

    let list = hydra_command()
        .args(["head", "list"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        list.status.success(),
        "Head list should succeed, stderr: {}",
        String::from_utf8_lossy(&list.stderr)
    );
    assert_eq!(
        String::from_utf8(list.stdout).expect("list output should be UTF-8"),
        "auth\npayment\n"
    );

    let status = hydra_command()
        .arg("status")
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        status.status.success(),
        "project status should succeed, stderr: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert_eq!(
        String::from_utf8(status.stdout).expect("status output should be UTF-8"),
        format!(
            "Project: {}\nHeads directory: {}\nHeads: 2\n  auth  clean\n  payment  clean\n",
            fs::canonicalize(&repository)
                .expect("repository should resolve")
                .display(),
            fs::canonicalize(heads_directory(&repository))
                .expect("Heads directory should resolve")
                .display()
        )
    );
    assert_eq!(
        fs::read(&state_path).expect("state should remain readable"),
        state_before,
        "read-only commands must not rewrite the inventory"
    );
    assert!(
        !head_state_lock_path(&repository).exists(),
        "read-only commands must not acquire the mutation lock"
    );
}

#[test]
fn head_status_reports_metadata_git_changes_and_ahead_behind() {
    let directory = TestDirectory::new("head-status");
    let repository = create_initialized_project(&directory);
    fs::write(repository.join("delete.txt"), b"delete me\n")
        .expect("tracked fixture should be written");
    commit(&repository, "add status fixtures");
    let base_commit = revision(&repository, "main");
    create_head(&repository, "payment");
    let head_path = heads_directory(&repository).join("payment");

    fs::write(head_path.join("committed.txt"), b"Head commit\n")
        .expect("committed fixture should be written");
    commit(&head_path, "Head progress");
    let head_commit = revision(&head_path, "HEAD");
    fs::write(repository.join("main-progress.txt"), b"Main progress\n")
        .expect("base fixture should be written");
    commit(&repository, "Main progress");

    fs::write(head_path.join("src/app.txt"), b"modified\n")
        .expect("modified fixture should be written");
    fs::remove_file(head_path.join("delete.txt")).expect("tracked fixture should be deleted");
    fs::write(head_path.join("added.txt"), b"staged addition\n")
        .expect("added fixture should be written");
    let output = run_git(&head_path, &["add", "added.txt"]);
    assert!(output.status.success());
    fs::write(head_path.join("untracked.txt"), b"untracked\n")
        .expect("untracked fixture should be written");

    let output = hydra_command()
        .args(["head", "status", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "Head status should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("status output should be UTF-8"),
        format!(
            "Head: payment\nPath: {}\nBranch: refs/heads/hydra/payment\nCommit: {head_commit}\nBase: refs/heads/main ({base_commit})\nTarget: refs/heads/main\nChanges: 1 modified, 1 added, 1 deleted, 1 untracked\nAhead/behind: 1/1\nWorktree: present\nConsistency: ok\n",
            fs::canonicalize(&head_path)
                .expect("Head path should resolve")
                .display()
        )
    );

    let project = hydra_command()
        .arg("status")
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");
    assert!(project.status.success());
    assert!(
        String::from_utf8(project.stdout)
            .expect("project status should be UTF-8")
            .contains("  payment  modified\n")
    );
}

#[test]
fn head_path_prints_only_the_recorded_absolute_path() {
    let directory = TestDirectory::new("head-path");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head_path = fs::canonicalize(heads_directory(&repository).join("payment"))
        .expect("Head should resolve");

    let output = hydra_command()
        .args(["head", "path", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("path output should be UTF-8"),
        format!("{}\n", head_path.display())
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn inspection_rejects_an_unknown_head_without_changing_state() {
    let directory = TestDirectory::new("head-unknown");
    let repository = create_initialized_project(&directory);
    let state_path = head_state_path(&repository);
    let state_before = fs::read(&state_path).expect("state should be readable");

    for arguments in [["head", "status", "missing"], ["head", "path", "missing"]] {
        let output = hydra_command()
            .args(arguments)
            .current_dir(&repository)
            .output()
            .expect("Hydra CLI should start");

        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).expect("error output should be UTF-8"),
            "error: Head \"missing\" does not exist\n"
        );
    }
    assert_eq!(
        fs::read(&state_path).expect("state should remain readable"),
        state_before
    );
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn status_reports_a_missing_worktree_as_an_inconsistency_without_repairing_it() {
    let directory = TestDirectory::new("head-missing-worktree");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head_path = heads_directory(&repository).join("payment");
    fs::remove_dir_all(&head_path).expect("fixture Head should be removed");

    let output = hydra_command()
        .args(["head", "status", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "an inspectable inconsistency should be reported, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("status output should be UTF-8");
    assert!(stdout.contains("Changes: unavailable\n"));
    assert!(stdout.contains("Worktree: missing\n"));
    assert!(stdout.contains("Consistency: worktree path is missing\n"));
    assert!(
        head_state_path(&repository).exists(),
        "status must not remove stale metadata"
    );
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn status_reports_a_missing_target_ref_as_an_inconsistency() {
    let directory = TestDirectory::new("head-missing-target");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let state_path = head_state_path(&repository);
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).expect("state should be readable"))
            .expect("state should be valid JSON");
    state["heads"]["payment"]["targetRef"] = "refs/heads/missing".into();
    fs::write(
        &state_path,
        serde_json::to_vec_pretty(&state).expect("state should serialize"),
    )
    .expect("state fixture should be updated");

    let output = hydra_command()
        .args(["head", "status", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .expect("status output should be UTF-8")
            .contains("Consistency: target ref is missing\n")
    );
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn status_falls_back_to_the_creation_commit_when_the_base_ref_is_missing() {
    let directory = TestDirectory::new("head-missing-base");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let state_path = head_state_path(&repository);
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).expect("state should be readable"))
            .expect("state should be valid JSON");
    state["heads"]["payment"]["baseRef"] = "refs/heads/missing".into();
    fs::write(
        &state_path,
        serde_json::to_vec_pretty(&state).expect("state should serialize"),
    )
    .expect("state fixture should be updated");

    let output = hydra_command()
        .args(["head", "status", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "the exact creation commit should remain comparable, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("status output should be UTF-8");
    assert!(stdout.contains("Ahead/behind: 0/0\n"));
    assert!(stdout.contains("Consistency: base ref is missing\n"));
}

#[test]
fn inspection_rejects_a_recorded_path_outside_the_owned_heads_directory() {
    let directory = TestDirectory::new("head-unsafe-recorded-path");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let state_path = head_state_path(&repository);
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).expect("state should be readable"))
            .expect("state should be valid JSON");
    state["heads"]["payment"]["worktreePath"] = repository.display().to_string().into();
    fs::write(
        &state_path,
        serde_json::to_vec_pretty(&state).expect("state should serialize"),
    )
    .expect("state fixture should be updated");

    let output = hydra_command()
        .args(["head", "path", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .expect("error output should be UTF-8")
            .contains("recorded Head path")
    );
    assert!(!head_state_lock_path(&repository).exists());
}

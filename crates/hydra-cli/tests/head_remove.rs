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

fn commit_all(repository: &Path, message: &str) {
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

#[test]
fn head_remove_deletes_a_clean_integrated_head_branch_and_inventory_entry() {
    let directory = TestDirectory::new("head-remove-clean");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");

    let output = hydra_command()
        .args(["head", "remove", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "clean removal should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be UTF-8"),
        "Removed Head payment\n"
    );
    assert!(!head.exists());
    assert!(!head_is_recorded(&repository, "payment"));
    assert!(!branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn head_remove_rejects_uncommitted_changes_without_mutation() {
    let directory = TestDirectory::new("head-remove-dirty");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("src/app.txt"), b"dirty\n").expect("tracked file should be changed");
    fs::write(head.join("untracked.txt"), b"local\n").expect("untracked file should be created");

    let output = hydra_command()
        .args(["head", "remove", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("uncommitted changes"));
    assert!(head.is_dir());
    assert!(head_is_recorded(&repository, "payment"));
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn head_remove_rejects_commits_not_integrated_into_the_target() {
    let directory = TestDirectory::new("head-remove-unintegrated");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"feature\n").expect("feature file should be written");
    commit_all(&head, "feature progress");

    let output = hydra_command()
        .args(["head", "remove", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not integrated"));
    assert!(head.is_dir());
    assert!(head_is_recorded(&repository, "payment"));
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn forced_removal_discards_worktree_changes_but_preserves_an_unintegrated_branch() {
    let directory = TestDirectory::new("head-remove-force");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let head = heads_directory(&repository).join("payment");
    fs::write(head.join("feature.txt"), b"committed\n").expect("feature file should be written");
    commit_all(&head, "recoverable feature");
    let commit = run_git(&head, &["rev-parse", "HEAD"]);
    assert!(commit.status.success());
    let commit = String::from_utf8(commit.stdout)
        .expect("commit should be UTF-8")
        .trim()
        .to_owned();
    fs::write(head.join("scratch.txt"), b"discarded\n").expect("scratch file should be written");

    let output = hydra_command()
        .args(["head", "remove", "payment", "--force"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "forced removal should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be UTF-8"),
        concat!(
            "Removed Head payment\n",
            "Preserved branch refs/heads/hydra/payment with unintegrated commits\n"
        )
    );
    assert!(!head.exists());
    assert!(!head_is_recorded(&repository, "payment"));
    assert!(branch_exists(&repository, "payment"));
    let preserved = run_git(&repository, &["rev-parse", "refs/heads/hydra/payment"]);
    assert!(preserved.status.success());
    assert_eq!(
        String::from_utf8(preserved.stdout)
            .expect("preserved commit should be UTF-8")
            .trim(),
        commit
    );
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn force_does_not_override_an_unsafe_recorded_path() {
    let directory = TestDirectory::new("head-remove-unsafe-path");
    let repository = create_initialized_project(&directory);
    create_head(&repository, "payment");
    let real_head = heads_directory(&repository).join("payment");
    let outside = directory.path().join("outside");
    fs::create_dir(&outside).expect("outside directory should be created");
    fs::write(outside.join("preserve.txt"), b"preserve\n")
        .expect("outside content should be written");
    let state_path = head_state_path(&repository);
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

    let output = hydra_command()
        .args(["head", "remove", "payment", "--force"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsafe"));
    assert_eq!(
        fs::read(outside.join("preserve.txt")).expect("outside content should remain"),
        b"preserve\n"
    );
    assert!(real_head.is_dir());
    assert!(branch_exists(&repository, "payment"));
    assert!(!head_state_lock_path(&repository).exists());
}

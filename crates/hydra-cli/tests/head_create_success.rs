mod common;

use std::{fs, process::Stdio};

use common::{
    TestDirectory, create_initialized_project, head_state_path, heads_directory, hydra_command,
    run_git,
};

fn relocate_heads_directory(
    repository: &std::path::Path,
    destination: &std::path::Path,
    policy: serde_json::Value,
) {
    let original = heads_directory(repository);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).expect("destination parent should be created");
    }
    fs::rename(&original, destination).expect("Heads directory should be relocated");
    let destination =
        fs::canonicalize(destination).expect("relocated Heads directory should resolve");

    let configuration_path = repository.join(".hydra.json");
    let mut configuration: serde_json::Value = serde_json::from_slice(
        &fs::read(&configuration_path).expect("configuration should be readable"),
    )
    .expect("configuration should be valid JSON");
    configuration["headsDirectory"] = policy;
    fs::write(
        &configuration_path,
        serde_json::to_vec_pretty(&configuration).expect("configuration should serialize"),
    )
    .expect("configuration should be updated");

    let locator_path = repository.join(".git/hydra/project.json");
    let mut locator: serde_json::Value =
        serde_json::from_slice(&fs::read(&locator_path).expect("locator should be readable"))
            .expect("locator should be valid JSON");
    locator["headsDirectory"] = destination.display().to_string().into();
    fs::write(
        &locator_path,
        serde_json::to_vec_pretty(&locator).expect("locator should serialize"),
    )
    .expect("locator should be updated");
}

#[test]
fn head_create_builds_an_isolated_worktree_and_records_its_metadata() {
    let directory = TestDirectory::new("head-create-success");
    let repository = create_initialized_project(&directory);
    let head_path = fs::canonicalize(directory.path().join("SampleProject.heads"))
        .expect("Heads directory should resolve")
        .join("payment");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "Head creation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("success output should be UTF-8");
    assert!(stdout.contains(&format!(
        "New Head successfully created at {}",
        head_path.display()
    )));
    assert!(
        stdout.contains("Storage backend: copy-on-write")
            || stdout.contains("Storage backend: full copy")
    );

    assert!(head_path.join(".git").is_file());
    assert_eq!(
        fs::read(head_path.join("src/app.txt")).expect("Head file should be readable"),
        b"base\n"
    );
    let branch = run_git(&head_path, &["branch", "--show-current"]);
    assert!(branch.status.success());
    assert_eq!(
        String::from_utf8_lossy(&branch.stdout).trim(),
        "hydra/payment"
    );
    let status = run_git(&head_path, &["status", "--porcelain"]);
    assert!(status.status.success());
    assert!(
        status.stdout.is_empty(),
        "new Head should be clean, got: {}",
        String::from_utf8_lossy(&status.stdout)
    );

    fs::write(head_path.join("src/app.txt"), b"head change\n")
        .expect("Head file should be writable");
    assert_eq!(
        fs::read(repository.join("src/app.txt")).expect("source file should remain readable"),
        b"base\n",
        "editing a Head must not modify the source workspace"
    );

    let state: serde_json::Value = serde_json::from_slice(
        &fs::read(head_state_path(&repository)).expect("state should be readable"),
    )
    .expect("state should remain valid JSON");
    let metadata = &state["heads"]["payment"];
    assert_eq!(metadata["worktreePath"], head_path.display().to_string());
    assert_eq!(metadata["headRef"], "refs/heads/hydra/payment");
    assert_eq!(metadata["baseRef"], "refs/heads/main");
    assert_eq!(metadata["targetRef"], "refs/heads/main");
    assert!(
        metadata["baseCommit"]
            .as_str()
            .is_some_and(|commit| commit.len() >= 40)
    );
    assert!(matches!(
        metadata["materializationBackend"].as_str(),
        Some("cow" | "copy")
    ));
    assert!(
        metadata["createdAt"]
            .as_str()
            .is_some_and(|timestamp| timestamp.ends_with('Z'))
    );
}

#[test]
fn head_create_resolves_explicit_base_and_target_branches() {
    let directory = TestDirectory::new("head-create-refs");
    let repository = create_initialized_project(&directory);
    let output = run_git(&repository, &["branch", "beta"]);
    assert!(output.status.success());

    let output = hydra_command()
        .args([
            "head", "create", "auth", "--from", "beta", "--target", "main",
        ])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "explicit refs should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let state: serde_json::Value = serde_json::from_slice(
        &fs::read(head_state_path(&repository)).expect("state should be readable"),
    )
    .expect("state should be valid JSON");
    assert_eq!(state["heads"]["auth"]["baseRef"], "refs/heads/beta");
    assert_eq!(state["heads"]["auth"]["targetRef"], "refs/heads/main");
}

#[test]
fn head_create_uses_the_same_heads_directory_when_invoked_from_a_head() {
    let directory = TestDirectory::new("head-create-from-head");
    let repository = create_initialized_project(&directory);
    let output = run_git(&repository, &["add", ".hydra.json"]);
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
            "track Hydra configuration",
        ],
    );
    assert!(output.status.success());

    let first = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");
    assert!(
        first.status.success(),
        "first Head creation should succeed, stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let payment = directory.path().join("SampleProject.heads/payment");
    let second = hydra_command()
        .args(["head", "create", "auth"])
        .current_dir(&payment)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        second.status.success(),
        "creation from a Head should use the installation locator, stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(directory.path().join("SampleProject.heads/auth").is_dir());
    assert!(
        !directory
            .path()
            .join("SampleProject.heads/payment.heads")
            .exists(),
        "the versioned sibling policy must not be resolved relative to the current Head"
    );
    let state: serde_json::Value = serde_json::from_slice(
        &fs::read(head_state_path(&repository)).expect("state should be readable"),
    )
    .expect("state should be valid JSON");
    assert!(state["heads"]["payment"].is_object());
    assert!(state["heads"]["auth"].is_object());
}

#[test]
fn head_create_accepts_an_arbitrary_safe_sibling_suffix() {
    let directory = TestDirectory::new("head-arbitrary-suffix");
    let repository = create_initialized_project(&directory);
    let relocated = directory.path().join("SampleProject custom heads 🚀");
    relocate_heads_directory(
        &repository,
        &relocated,
        serde_json::json!({
            "strategy": "sibling",
            "suffix": " custom heads 🚀"
        }),
    );

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "safe Unicode and whitespace should be accepted in a suffix, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(relocated.join("payment").is_dir());
}

#[test]
fn head_create_resolves_a_portable_relative_heads_directory() {
    let directory = TestDirectory::new("head-relative-strategy");
    let repository = create_initialized_project(&directory);
    let relocated = directory
        .path()
        .join("shared workspaces/SampleProject heads");
    relocate_heads_directory(
        &repository,
        &relocated,
        serde_json::json!({
            "strategy": "relative",
            "base": "repositoryParent",
            "path": "shared workspaces/SampleProject heads"
        }),
    );

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "relative strategy should resolve from the stable project parent, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(relocated.join("payment").is_dir());
}

#[test]
fn head_create_uses_a_non_versioned_local_heads_directory() {
    let directory = TestDirectory::new("head-local-strategy");
    let repository = create_initialized_project(&directory);
    let relocated = directory.path().join("device-specific storage");
    relocate_heads_directory(
        &repository,
        &relocated,
        serde_json::json!({
            "strategy": "local"
        }),
    );

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "local strategy should trust only the verified non-versioned locator, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(relocated.join("payment").is_dir());
}

#[test]
fn head_create_materializes_tracked_content_from_the_resolved_commit() {
    let directory = TestDirectory::new("head-create-base-content");
    let repository = create_initialized_project(&directory);
    fs::write(
        repository.join("src/app.txt"),
        b"uncommitted source change\n",
    )
    .expect("source file should be changed");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "Head creation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(
            directory
                .path()
                .join("SampleProject.heads/payment/src/app.txt")
        )
        .expect("Head file should be readable"),
        b"base\n",
        "tracked content must come from baseCommit, not the source working tree"
    );
    assert_eq!(
        fs::read(repository.join("src/app.txt")).expect("source file should remain readable"),
        b"uncommitted source change\n"
    );
}

#[test]
fn head_create_materializes_cow_gitignore_overlays_without_confirmation() {
    let directory = TestDirectory::new("head-create-overlay");
    let repository = create_initialized_project(&directory);
    fs::write(
        repository.join(".gitignore"),
        b".env\ncache/\n!cache/logs/\n",
    )
    .expect("gitignore should be written");
    let output = run_git(&repository, &["add", ".gitignore"]);
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
            "add overlay rules",
        ],
    );
    assert!(output.status.success());

    fs::write(repository.join(".env"), b"secret\n").expect("overlay file should be written");
    fs::create_dir_all(repository.join("cache/logs")).expect("overlay directories should be made");
    fs::write(repository.join("cache/data.bin"), b"cache\n")
        .expect("overlay file should be written");
    fs::write(repository.join("cache/logs/skip.log"), b"skip\n")
        .expect("excluded overlay should be written");

    let output = hydra_command()
        .args(["head", "create", "overlay"])
        .current_dir(&repository)
        .stdin(Stdio::null())
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "copy-on-write overlay should succeed without input, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("output should be UTF-8");
    assert!(stdout.contains("Overlay: 2 file(s), 13 byte(s)"));
    assert!(
        !stdout.contains("[y/N]"),
        "copy-on-write overlays should not prompt, got: {stdout:?}"
    );

    let head_path = directory.path().join("SampleProject.heads/overlay");
    assert_eq!(
        fs::read(head_path.join(".env")).expect("overlay should be readable"),
        b"secret\n"
    );
    assert_eq!(
        fs::read(head_path.join("cache/data.bin")).expect("overlay should be readable"),
        b"cache\n"
    );
    assert!(!head_path.join("cache/logs/skip.log").exists());

    fs::write(head_path.join(".env"), b"head secret\n").expect("Head overlay should be writable");
    assert_eq!(
        fs::read(repository.join(".env")).expect("source overlay should remain readable"),
        b"secret\n",
        "editing a Head overlay must not modify the source workspace"
    );
    let status = run_git(&head_path, &["status", "--porcelain"]);
    assert!(status.status.success());
    assert!(
        status.stdout.is_empty(),
        "ignored overlays should keep the Head clean"
    );
}

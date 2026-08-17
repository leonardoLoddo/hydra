mod common;

use std::{fs, io::Write, path::Path, process::Stdio};

use common::{
    TestDirectory, create_initialized_project, head_state_path, heads_directory, hydra_command,
    overlay_copy_on_write_is_supported, run_git,
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

fn commit_all(repository: &Path, message: &str) {
    let output = run_git(repository, &["add", "--all"]);
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
    assert!(output.status.success());
}

fn create_head_confirming_fallback(repository: &Path, name: &str) -> std::process::Output {
    let mut child = hydra_command()
        .args(["head", "create", name])
        .current_dir(repository)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Hydra CLI should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(b"yes\n")
        .expect("full-copy confirmation should be written");
    child.wait_with_output().expect("Hydra should finish")
}

fn create_head(repository: &Path, name: &str) {
    let output = create_head_confirming_fallback(repository, name);
    assert!(
        output.status.success(),
        "Head creation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn close_head_from(head: &Path, name: &str) {
    let output = hydra_command()
        .args(["head", "close", name])
        .current_dir(head)
        .output()
        .expect("Hydra CLI should start");
    assert!(
        output.status.success(),
        "Head {name} should close from itself, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_status_uses_parent(head: &Path, repository: &Path) {
    let output = hydra_command()
        .arg("status")
        .current_dir(head)
        .output()
        .expect("Hydra CLI should start");
    assert!(
        output.status.success(),
        "status from a Head should use the parent configuration, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parent = fs::canonicalize(repository).expect("repository should resolve");
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .starts_with(&format!("Project: {}\n", parent.display())),
        "status should identify the canonical parent project"
    );
}

fn assert_auth_uses_parent_context(repository: &Path, auth: &Path, parent_commit: &str) {
    let state: serde_json::Value = serde_json::from_slice(
        &fs::read(head_state_path(repository)).expect("state should be readable"),
    )
    .expect("state should be valid JSON");
    assert!(state["heads"]["payment"].is_object());
    assert!(state["heads"]["auth"].is_object());
    assert_eq!(state["heads"]["auth"]["baseRef"], "refs/heads/main");
    assert_eq!(state["heads"]["auth"]["baseCommit"], parent_commit);
    assert_eq!(state["heads"]["auth"]["targetRef"], "refs/heads/main");
    assert_eq!(
        fs::read(auth.join("src/app.txt")).expect("auth source should be readable"),
        b"base\n",
        "tracked files should come from the parent project's HEAD"
    );
    assert_eq!(
        fs::read(auth.join(".env")).expect("auth overlay should be readable"),
        b"parent overlay\n",
        "overlays should come from the parent project rather than the calling Head"
    );
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
    assert!(
        !heads_directory(&repository)
            .join(".hydra/pending-payment.json")
            .exists(),
        "committed creation must remove its pending journal"
    );
}

#[test]
fn head_create_can_force_full_copy_for_tracked_files_and_overlays() {
    let directory = TestDirectory::new("head-create-full-copy");
    let repository = create_initialized_project(&directory);
    fs::write(repository.join(".gitignore"), b".env\n").expect("overlay rules should be written");
    commit_all(&repository, "add overlay rules");
    fs::write(repository.join(".env"), b"secret\n").expect("overlay should be written");

    let configuration_path = repository.join(".hydra.json");
    let mut configuration: serde_json::Value = serde_json::from_slice(
        &fs::read(&configuration_path).expect("configuration should be readable"),
    )
    .expect("configuration should be valid JSON");
    configuration["storage"]["mode"] = "copy".into();
    fs::write(
        &configuration_path,
        serde_json::to_vec_pretty(&configuration).expect("configuration should serialize"),
    )
    .expect("copy mode should be configured");

    let mut child = hydra_command()
        .args(["head", "create", "copy-mode"])
        .current_dir(&repository)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Hydra CLI should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(b"yes\n")
        .expect("full-copy confirmation should be written");
    let output = child.wait_with_output().expect("Hydra should finish");

    assert!(
        output.status.success(),
        "forced full-copy creation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("Full copy required: 1 file(s), 7 byte(s)"));
    assert!(stdout.contains("Storage backend: full copy"));

    let head = heads_directory(&repository).join("copy-mode");
    assert_eq!(
        fs::read(head.join("src/app.txt")).expect("tracked file should be readable"),
        b"base\n"
    );
    assert_eq!(
        fs::read(head.join(".env")).expect("overlay should be readable"),
        b"secret\n"
    );
    let state: serde_json::Value = serde_json::from_slice(
        &fs::read(head_state_path(&repository)).expect("state should be readable"),
    )
    .expect("state should be valid JSON");
    assert_eq!(
        state["heads"]["copy-mode"]["materializationBackend"],
        "copy"
    );

    fs::write(head.join("src/app.txt"), b"head tracked\n")
        .expect("tracked Head file should be writable");
    fs::write(head.join(".env"), b"head overlay\n").expect("overlay Head file should be writable");
    assert_eq!(
        fs::read(repository.join("src/app.txt")).expect("source tracked file should be readable"),
        b"base\n"
    );
    assert_eq!(
        fs::read(repository.join(".env")).expect("source overlay should be readable"),
        b"secret\n"
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
fn head_create_rejects_the_unpublished_json_schema_annotation() {
    let directory = TestDirectory::new("head-schema-annotation");
    let repository = create_initialized_project(&directory);
    let configuration_path = repository.join(".hydra.json");
    let mut configuration: serde_json::Value = serde_json::from_slice(
        &fs::read(&configuration_path).expect("configuration should be readable"),
    )
    .expect("configuration should be valid JSON");
    configuration["$schema"] = "https://example.invalid/hydra.schema.json".into();
    fs::write(
        &configuration_path,
        serde_json::to_vec_pretty(&configuration).expect("configuration should serialize"),
    )
    .expect("schema-annotated configuration should be written");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown field `$schema`"),
        "the error should identify the unsupported annotation, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !heads_directory(&repository).join("payment").exists(),
        "invalid configuration must not create a Head"
    );
}

#[test]
fn commands_from_a_head_use_the_parent_project_context() {
    let directory = TestDirectory::new("head-create-from-head");
    let repository = create_initialized_project(&directory);
    fs::write(repository.join(".gitignore"), b".env\n").expect("overlay rules should be written");
    fs::write(repository.join(".env"), b"parent overlay\n")
        .expect("parent overlay should be written");
    commit_all(&repository, "track Hydra configuration");
    let output = run_git(&repository, &["rev-parse", "main"]);
    assert!(output.status.success());
    let parent_commit = String::from_utf8(output.stdout)
        .expect("parent commit should be UTF-8")
        .trim()
        .to_owned();

    create_head(&repository, "payment");
    let payment = directory.path().join("SampleProject.heads/payment");
    fs::write(payment.join("src/app.txt"), b"payment progress\n")
        .expect("payment progress should be written");
    commit_all(&payment, "payment progress");
    fs::write(payment.join(".env"), b"payment overlay\n")
        .expect("payment overlay should be changed");
    fs::remove_file(payment.join(".hydra.json"))
        .expect("Head configuration should be removable for discovery coverage");

    assert_status_uses_parent(&payment, &repository);
    create_head(&payment, "auth");
    assert!(directory.path().join("SampleProject.heads/auth").is_dir());
    assert!(
        !directory
            .path()
            .join("SampleProject.heads/payment.heads")
            .exists(),
        "the versioned sibling policy must not be resolved relative to the current Head"
    );
    let auth = directory.path().join("SampleProject.heads/auth");
    assert_auth_uses_parent_context(&repository, &auth, &parent_commit);

    let output = run_git(&payment, &["restore", ".hydra.json"]);
    assert!(output.status.success());
    close_head_from(&payment, "payment");

    let auth_status = hydra_command()
        .args(["head", "status", "auth"])
        .current_dir(&auth)
        .output()
        .expect("Hydra CLI should start");
    assert!(
        auth_status.status.success(),
        "the sibling should remain consistent after closing the calling Head: {}",
        String::from_utf8_lossy(&auth_status.stderr)
    );
    assert!(String::from_utf8_lossy(&auth_status.stdout).contains("Consistency: ok"));
    close_head_from(&auth, "auth");
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

#[cfg(unix)]
#[test]
fn head_create_reuses_clean_tracked_working_files_when_copy_on_write_is_available() {
    use std::{env, os::unix::fs::PermissionsExt};

    let directory = TestDirectory::new("head-create-reuse-tracked");
    let repository = create_initialized_project(&directory);
    let wrapper_directory = directory.path().join("git-wrapper");
    fs::create_dir(&wrapper_directory).expect("Git wrapper directory should be created");
    let wrapper = wrapper_directory.join("git");
    fs::write(
        &wrapper,
        concat!(
            "#!/bin/sh\n",
            "printf '%s\\n' \"$*\" >> \"$HYDRA_TEST_GIT_LOG\"\n",
            "exec \"$HYDRA_TEST_REAL_GIT\" \"$@\"\n"
        ),
    )
    .expect("Git wrapper should be written");
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755))
        .expect("Git wrapper should be executable");
    let original_path = env::var_os("PATH").expect("test PATH should exist");
    let real_git = env::split_paths(&original_path)
        .map(|directory| directory.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("real Git should be present on PATH");
    let wrapped_path = env::join_paths(
        std::iter::once(wrapper_directory.clone()).chain(env::split_paths(&original_path)),
    )
    .expect("wrapped PATH should be valid");
    let log = directory.path().join("git.log");

    let output = hydra_command()
        .args(["head", "create", "reused"])
        .current_dir(&repository)
        .env("PATH", wrapped_path)
        .env("HYDRA_TEST_REAL_GIT", &real_git)
        .env("HYDRA_TEST_GIT_LOG", &log)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "Head creation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let commands = fs::read_to_string(&log).expect("Git command log should be readable");
    let stdout = String::from_utf8(output.stdout).expect("success output should be UTF-8");
    if stdout.contains("Storage backend: copy-on-write") {
        assert!(
            !commands.lines().any(|command| command.contains("cat-file")),
            "clean tracked files should be cloned from the working tree, got:\n{commands}"
        );
    } else {
        assert!(
            commands.lines().any(|command| command.contains("cat-file")),
            "a volume without copy-on-write should read the committed blob"
        );
    }
    assert_eq!(
        fs::read(heads_directory(&repository).join("reused/src/app.txt"))
            .expect("reused tracked file should be readable"),
        b"base\n"
    );
}

#[cfg(unix)]
#[test]
fn head_create_preserves_tracked_executable_files_and_symlink_payloads() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let directory = TestDirectory::new("head-create-tracked-modes");
    let repository = create_initialized_project(&directory);
    let executable = repository.join("src/tool");
    fs::write(&executable, b"#!/bin/sh\nexit 0\n").expect("executable fixture should be written");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
        .expect("executable fixture mode should be set");
    symlink("tool", repository.join("src/tool-link"))
        .expect("tracked symlink fixture should be created");
    assert!(
        run_git(&repository, &["add", "src/tool", "src/tool-link"])
            .status
            .success()
    );
    assert!(
        run_git(
            &repository,
            &[
                "-c",
                "user.name=Hydra Tests",
                "-c",
                "user.email=hydra-tests@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "add tracked modes",
            ],
        )
        .status
        .success()
    );

    let output = hydra_command()
        .args(["head", "create", "tracked-modes"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "Head creation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let head = heads_directory(&repository).join("tracked-modes");
    assert_eq!(
        fs::read(head.join("src/tool")).expect("tracked executable should be readable"),
        b"#!/bin/sh\nexit 0\n"
    );
    assert_ne!(
        fs::metadata(head.join("src/tool"))
            .expect("tracked executable metadata should be readable")
            .permissions()
            .mode()
            & 0o111,
        0
    );
    assert_eq!(
        fs::read_link(head.join("src/tool-link")).expect("tracked symlink should be readable"),
        std::path::PathBuf::from("tool")
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

    if !overlay_copy_on_write_is_supported(&repository, &repository.join(".env")) {
        eprintln!("skipping native CoW overlay test: test volume does not support reflinks");
        return;
    }

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
    assert!(
        output.stderr.is_empty(),
        "non-interactive progress must not pollute stderr"
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

#[cfg(unix)]
#[test]
fn head_create_preserves_a_safe_relative_overlay_symlink() {
    use std::os::unix::fs::symlink;

    let directory = TestDirectory::new("head-create-overlay-symlink");
    let repository = create_initialized_project(&directory);
    fs::write(repository.join(".gitignore"), b"node_modules/\n")
        .expect("overlay rules should be written");
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
            "add dependency overlay rules",
        ],
    );
    assert!(output.status.success());

    let dependency = repository.join("node_modules/acorn/bin/acorn");
    fs::create_dir_all(
        dependency
            .parent()
            .expect("dependency should have a parent"),
    )
    .expect("dependency directories should be created");
    fs::create_dir_all(repository.join("node_modules/.bin"))
        .expect("binary directory should be created");
    fs::write(&dependency, b"source dependency\n").expect("dependency should be written");
    symlink(
        "../acorn/bin/acorn",
        repository.join("node_modules/.bin/acorn"),
    )
    .expect("dependency symlink should be created");

    let output = create_head_confirming_fallback(&repository, "dependencies");

    assert!(
        output.status.success(),
        "safe relative overlay symlink should be materialized, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let head_path = directory.path().join("SampleProject.heads/dependencies");
    let head_symlink = head_path.join("node_modules/.bin/acorn");
    assert_eq!(
        fs::read_link(&head_symlink).expect("Head symlink should be readable"),
        std::path::PathBuf::from("../acorn/bin/acorn")
    );
    assert_eq!(
        fs::read(&head_symlink).expect("Head symlink target should be readable"),
        b"source dependency\n"
    );

    fs::write(&dependency, b"changed source dependency\n")
        .expect("source dependency should remain writable");
    assert_eq!(
        fs::read(&head_symlink).expect("Head dependency should remain readable"),
        b"source dependency\n",
        "the Head symlink must resolve to the isolated Head dependency"
    );

    let status = run_git(&head_path, &["status", "--porcelain"]);
    assert!(status.status.success());
    assert!(
        status.stdout.is_empty(),
        "ignored symlink overlays should keep the Head clean"
    );
}

#[cfg(unix)]
#[test]
fn head_create_preserves_overlay_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TestDirectory::new("head-create-overlay-permissions");
    let repository = create_initialized_project(&directory);
    fs::write(repository.join(".gitignore"), b"tools/\n").expect("overlay rules should be written");
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
            "add executable overlay rules",
        ],
    );
    assert!(output.status.success());

    let executable = repository.join("tools/run");
    fs::create_dir(repository.join("tools")).expect("tools directory should be created");
    fs::write(&executable, b"#!/bin/sh\n").expect("executable overlay should be written");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o751))
        .expect("overlay permissions should be set");

    let output = create_head_confirming_fallback(&repository, "permissions");

    assert!(
        output.status.success(),
        "executable overlay should be materialized, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let head_executable = directory
        .path()
        .join("SampleProject.heads/permissions/tools/run");
    let mode = fs::metadata(&head_executable)
        .expect("Head executable should exist")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o751);
}

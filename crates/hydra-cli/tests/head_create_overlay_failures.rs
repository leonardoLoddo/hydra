mod common;

use std::{fs, io::Write, process::Stdio};

use common::{
    TestDirectory, assert_no_head_creation_artifacts, create_initialized_project, heads_directory,
    hydra_command, run_git,
};

#[test]
fn head_create_does_not_prompt_only_because_overlays_are_present() {
    let directory = TestDirectory::new("head-overlay-no-prompt");
    let repository = create_initialized_project(&directory);
    fs::write(repository.join(".gitignore"), b".env\n").expect("overlay rules should be written");
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
    fs::write(repository.join(".env"), b"secret\n").expect("overlay should be written");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .stdin(Stdio::null())
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "copy-on-write overlay should not need confirmation, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("output should be UTF-8");
    assert!(stdout.contains("Overlay: 1 file(s), 7 byte(s)"));
    assert!(!stdout.contains("[y/N]"));
}

#[test]
fn head_create_rejects_an_overlay_that_would_overwrite_a_tracked_file() {
    let directory = TestDirectory::new("head-overlay-tracked");
    let repository = create_initialized_project(&directory);
    fs::write(repository.join(".gitignore"), b"src/app.txt\n")
        .expect("overlay rules should be written");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("overwrite a tracked file"));
    assert_no_head_creation_artifacts(&repository, "payment");
}

#[cfg(unix)]
#[test]
fn head_create_requires_confirmation_before_excluding_an_absolute_overlay_symlink() {
    use std::os::unix::fs::symlink;

    let directory = TestDirectory::new("head-overlay-symlink");
    let repository = create_initialized_project(&directory);
    let outside = directory.path().join("outside-secret");
    fs::write(&outside, b"outside\n").expect("outside file should be written");
    fs::write(repository.join(".gitignore"), b"escape\n").expect("overlay rules should be written");
    symlink(&outside, repository.join("escape")).expect("overlay symlink should be created");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("escape"));
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("Exclude them and update .hydra.json? [y/N]")
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Head creation cancelled"));
    assert_no_head_creation_artifacts(&repository, "payment");
    assert_eq!(
        fs::read(outside).expect("outside file should remain readable"),
        b"outside\n"
    );
}

#[cfg(unix)]
#[test]
fn head_create_requires_confirmation_before_excluding_an_escaping_overlay_symlink() {
    use std::os::unix::fs::symlink;

    let directory = TestDirectory::new("head-overlay-relative-symlink-escape");
    let repository = create_initialized_project(&directory);
    let outside = directory.path().join("outside-secret");
    fs::write(&outside, b"outside\n").expect("outside file should be written");
    fs::write(repository.join(".gitignore"), b"links/\n").expect("overlay rules should be written");
    fs::create_dir(repository.join("links")).expect("overlay directory should be created");
    symlink("../../outside-secret", repository.join("links/escape"))
        .expect("overlay symlink should be created");

    let output = hydra_command()
        .args(["head", "create", "payment"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("links/escape"));
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("Exclude them and update .hydra.json? [y/N]")
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Head creation cancelled"));
    assert_no_head_creation_artifacts(&repository, "payment");
    assert_eq!(
        fs::read(outside).expect("outside file should remain readable"),
        b"outside\n"
    );
}

#[cfg(unix)]
#[test]
fn head_create_can_exclude_all_unsafe_overlay_symlinks_and_update_configuration() {
    use std::os::unix::fs::symlink;

    let directory = TestDirectory::new("head-overlay-exclude-unsafe-symlinks");
    let repository = create_initialized_project(&directory);
    let outside = directory.path().join("outside-secret");
    fs::write(&outside, b"outside\n").expect("outside file should be written");
    fs::create_dir_all(repository.join("public")).expect("public directory should be created");
    fs::create_dir_all(repository.join("storage/app/public"))
        .expect("storage target should be created");
    fs::create_dir_all(repository.join("links")).expect("links directory should be created");
    fs::write(
        repository.join(".gitignore"),
        b"public/storage\nlinks/escape\n",
    )
    .expect("overlay rules should be written");
    let absolute_target = repository.join("storage/app/public");
    symlink(&absolute_target, repository.join("public/storage"))
        .expect("absolute internal symlink should be created");
    symlink("../../outside-secret", repository.join("links/escape"))
        .expect("escaping symlink should be created");

    let mut child = hydra_command()
        .args(["head", "create", "payment"])
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
        .expect("confirmation should be written");
    let output = child.wait_with_output().expect("Hydra should finish");

    assert!(
        output.status.success(),
        "confirmed exclusion should create the Head, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("Unsafe overlay symlinks:"));
    assert!(stdout.contains("links/escape"));
    assert!(stdout.contains("public/storage"));
    assert!(stdout.contains("Exclude them and update .hydra.json? [y/N]"));

    let configuration: serde_json::Value = serde_json::from_slice(
        &fs::read(repository.join(".hydra.json")).expect("configuration should be readable"),
    )
    .expect("configuration should remain valid JSON");
    assert_eq!(
        configuration["overlay"]["copy"],
        serde_json::json!(["... .gitignore", "!/links/escape", "!/public/storage"])
    );

    let head = heads_directory(&repository).join("payment");
    assert!(head.is_dir());
    assert!(!head.join("links/escape").exists());
    assert!(!head.join("public/storage").exists());
    assert_eq!(
        fs::read(&outside).expect("outside file should remain readable"),
        b"outside\n"
    );
}

#[cfg(unix)]
#[test]
fn declining_unsafe_symlink_exclusion_preserves_configuration_and_git_state() {
    use std::os::unix::fs::symlink;

    let directory = TestDirectory::new("head-overlay-decline-unsafe-symlink");
    let repository = create_initialized_project(&directory);
    let outside = directory.path().join("outside-secret");
    fs::write(&outside, b"outside\n").expect("outside file should be written");
    fs::write(repository.join(".gitignore"), b"escape\n").expect("overlay rules should be written");
    symlink(&outside, repository.join("escape")).expect("overlay symlink should be created");
    let configuration_path = repository.join(".hydra.json");
    let original_configuration =
        fs::read(&configuration_path).expect("configuration should be readable");

    let mut child = hydra_command()
        .args(["head", "create", "payment"])
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
        .write_all(b"no\n")
        .expect("decline response should be written");
    let output = child.wait_with_output().expect("Hydra should finish");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("Exclude them and update .hydra.json? [y/N]")
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Head creation cancelled"));
    assert_eq!(
        fs::read(configuration_path).expect("configuration should remain readable"),
        original_configuration
    );
    assert_no_head_creation_artifacts(&repository, "payment");
}

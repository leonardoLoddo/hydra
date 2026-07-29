mod common;

use std::{fs, process::Stdio};

use common::{
    TestDirectory, assert_no_head_creation_artifacts, create_initialized_project, hydra_command,
    run_git,
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
fn head_create_rejects_an_absolute_overlay_symlink() {
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
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsafe"));
    assert_no_head_creation_artifacts(&repository, "payment");
    assert_eq!(
        fs::read(outside).expect("outside file should remain readable"),
        b"outside\n"
    );
}

#[cfg(unix)]
#[test]
fn head_create_rejects_a_relative_overlay_symlink_that_escapes_the_project() {
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
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsafe"));
    assert_no_head_creation_artifacts(&repository, "payment");
    assert_eq!(
        fs::read(outside).expect("outside file should remain readable"),
        b"outside\n"
    );
}

mod common;

use std::{fs, io::Write, process::Stdio};

use common::{
    TestDirectory, assert_no_head_creation_artifacts, create_initialized_project, hydra_command,
};

#[test]
fn head_create_cancels_cleanly_when_overlays_are_not_confirmed() {
    let directory = TestDirectory::new("head-overlay-declined");
    let repository = create_initialized_project(&directory);
    fs::write(repository.join(".gitignore"), b".env\n").expect("overlay rules should be written");
    fs::write(repository.join(".env"), b"secret\n").expect("overlay should be written");

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
        .expect("Hydra stdin should be piped")
        .write_all(b"n\n")
        .expect("decline response should be written");
    let output = child.wait_with_output().expect("Hydra should finish");

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("output should be UTF-8");
    assert!(stdout.contains("Overlay: 1 file(s), 7 byte(s)"));
    assert!(stdout.contains("Copy these overlay files? [y/N]"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cancelled"));
    assert_no_head_creation_artifacts(&repository, "payment");
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
fn head_create_rejects_a_selected_overlay_symlink() {
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

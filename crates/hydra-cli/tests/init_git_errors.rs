mod common;

#[cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt};

#[cfg(target_os = "linux")]
use std::{ffi::OsString, os::unix::ffi::OsStringExt, process::Command};

use common::{TestDirectory, hydra_command};

#[cfg(target_os = "linux")]
#[test]
fn init_rejects_a_repository_name_that_cannot_be_stored_losslessly() {
    let directory = TestDirectory::new("non-utf8-name");
    let repository = directory
        .path()
        .join(OsString::from_vec(b"SampleProject-\xff".to_vec()));
    fs::create_dir(&repository).expect("repository directory should be created");

    let git = Command::new("git")
        .args(["init", "--quiet"])
        .arg(&repository)
        .status()
        .expect("Git should start");
    assert!(git.success(), "temporary Git repository should be created");

    let output = hydra_command()
        .arg("init")
        .arg(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("UTF-8"),
        "error should explain why the repository name is unsupported"
    );
    assert!(
        !repository.join(".hydra.json").exists(),
        "unsupported names must be rejected before mutation"
    );
    assert!(
        !repository.join(".git/hydra/heads.json").exists(),
        "unsupported names must not create local state"
    );
}

#[cfg(unix)]
#[test]
fn init_reports_a_git_command_failure_without_calling_it_a_missing_repository() {
    let directory = TestDirectory::new("git-failure");
    let fake_bin = directory.path().join("bin");
    fs::create_dir(&fake_bin).expect("fake binary directory should be created");

    let fake_git = fake_bin.join("git");
    fs::write(
        &fake_git,
        b"#!/bin/sh\nprintf 'simulated Git option failure\\n' >&2\nexit 129\n",
    )
    .expect("fake Git should be written");
    let mut permissions = fs::metadata(&fake_git)
        .expect("fake Git metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_git, permissions).expect("fake Git should be executable");

    let output = hydra_command()
        .arg("init")
        .arg(directory.path())
        .env("PATH", &fake_bin)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("error output should be UTF-8");
    assert!(
        stderr.contains("repository root"),
        "error should identify the failed Git operation, got: {stderr:?}"
    );
    assert!(
        stderr.contains("status 129"),
        "error should preserve the Git exit status, got: {stderr:?}"
    );
    assert!(
        stderr.contains("simulated Git option failure"),
        "error should preserve Git stderr, got: {stderr:?}"
    );
    assert!(
        !stderr.contains("not a Git repository"),
        "unrelated Git failures must not be misclassified, got: {stderr:?}"
    );
}

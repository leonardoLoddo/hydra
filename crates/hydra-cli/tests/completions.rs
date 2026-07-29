mod common;

use std::fs;

use common::{
    TestDirectory, create_initialized_project, head_state_lock_path, head_state_path, hydra_command,
};

#[test]
fn completions_prints_registration_for_supported_shells() {
    for (shell, expected) in [
        ("bash", "COMPLETE=bash hydra"),
        ("zsh", "COMPLETE=zsh hydra"),
        ("fish", "COMPLETE=fish hydra"),
    ] {
        let output = hydra_command()
            .args(["completions", shell])
            .output()
            .expect("Hydra CLI should start");

        assert!(
            output.status.success(),
            "{shell} completion registration should succeed, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("completion output should be UTF-8");
        assert!(
            stdout.contains(expected),
            "{shell} registration should activate Hydra's dynamic completion, got: {stdout:?}"
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn completions_rejects_unsupported_shells() {
    let output = hydra_command()
        .args(["completions", "powershell"])
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("error output should be UTF-8");
    assert!(stderr.contains("invalid value"));
    assert!(stderr.contains("bash"));
    assert!(stderr.contains("zsh"));
    assert!(stderr.contains("fish"));
}

#[test]
fn internal_head_candidates_are_sorted_unique_and_read_only() {
    let directory = TestDirectory::new("completion-candidates");
    let repository = create_initialized_project(&directory);

    for name in ["zeta", "alpha"] {
        let output = hydra_command()
            .current_dir(&repository)
            .args(["head", "create", name])
            .output()
            .expect("Hydra CLI should start");
        assert!(
            output.status.success(),
            "Head creation should succeed, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let inventory_path = head_state_path(&repository);
    let inventory_before =
        fs::read(&inventory_path).expect("Head inventory should be readable before completion");
    let output = hydra_command()
        .current_dir(&repository)
        .args(["__complete", "heads"])
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "candidate lookup should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("candidate output should be UTF-8"),
        "alpha\nzeta\n"
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        fs::read(&inventory_path).expect("Head inventory should remain readable"),
        inventory_before,
        "completion must not rewrite Hydra state"
    );
    assert!(
        !head_state_lock_path(&repository).exists(),
        "completion must not acquire a persistent state lock"
    );

    let output = hydra_command()
        .current_dir(&repository)
        .env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "3")
        .env("_CLAP_COMPLETE_COMP_TYPE", "9")
        .env("_CLAP_COMPLETE_SPACE", "true")
        .env("_CLAP_IFS", "\n")
        .args(["--", "hydra", "head", "status", "a"])
        .output()
        .expect("Hydra dynamic completion should start");
    assert!(
        output.status.success(),
        "dynamic completion should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("dynamic candidates should be UTF-8"),
        "alpha"
    );
}

#[test]
fn internal_head_candidates_are_silent_outside_a_hydra_project() {
    let directory = TestDirectory::new("completion-outside-project");
    let output = hydra_command()
        .current_dir(directory.path())
        .args(["__complete", "heads"])
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn internal_completion_protocol_is_hidden_from_public_help() {
    let output = hydra_command()
        .arg("--help")
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");
    assert!(stdout.contains("completions"));
    assert!(!stdout.contains("__complete"));
}

use std::process::Command;

#[test]
fn help_describes_hydra_and_its_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .arg("--help")
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");
    assert!(
        stdout.contains("Git-native"),
        "help should explain Hydra's purpose, got: {stdout:?}"
    );
    assert!(
        stdout.contains("Usage: hydra"),
        "help should show how to invoke Hydra, got: {stdout:?}"
    );
    assert!(
        stdout.contains("Command syntax:")
            && stdout.contains("hydra head create <NAME> [--from <REF>] [--target <BRANCH>]"),
        "top-level help should expose the complete create syntax, got: {stdout:?}"
    );
}

#[test]
fn version_reports_the_workspace_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .arg("--version")
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "version should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("version output should be UTF-8"),
        concat!("hydra ", env!("CARGO_PKG_VERSION"), "\n")
    );
}

#[test]
fn init_help_uses_the_documented_optional_path_syntax() {
    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["init", "--help"])
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");
    assert!(
        stdout.contains("Usage: hydra init [PATH]"),
        "init help should expose the documented syntax, got: {stdout:?}"
    );
}

#[test]
fn head_create_help_uses_the_documented_nested_syntax() {
    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["head", "create", "--help"])
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "head create help should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");
    assert!(
        stdout.contains("Usage: hydra head create [OPTIONS] <NAME>"),
        "help should expose the documented nested syntax, got: {stdout:?}"
    );
    assert!(
        stdout.contains("Create a new isolated Head"),
        "help should state the command outcome, got: {stdout:?}"
    );
    assert!(stdout.contains("--from <REF>"));
    assert!(
        stdout.contains("HEAD"),
        "help should document the default base, got: {stdout:?}"
    );
    assert!(
        stdout.contains("starts at <REF>") && stdout.contains("uses <BRANCH>"),
        "help should use the public option placeholders consistently, got: {stdout:?}"
    );
    assert!(
        stdout.contains("canonical parent project"),
        "help should explain that invocation from a Head uses the parent context, got: {stdout:?}"
    );
    assert!(stdout.contains("--target <BRANCH>"));
    assert!(
        stdout.contains("local branch"),
        "help should use familiar Git terminology, got: {stdout:?}"
    );
    assert!(
        stdout.contains("Examples:")
            && stdout.contains("hydra head create payment")
            && stdout.contains("hydra head create payment --from beta --target main"),
        "help should include copyable examples, got: {stdout:?}"
    );
}

#[test]
fn head_help_exposes_the_complete_create_syntax() {
    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["head", "--help"])
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "head help should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");
    assert!(
        stdout.contains("Command syntax:")
            && stdout.contains("hydra head create <NAME> [--from <REF>] [--target <BRANCH>]"),
        "head help should expose the complete create syntax, got: {stdout:?}"
    );
}

#[test]
fn help_exposes_the_complete_inspection_syntax() {
    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .arg("--help")
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");
    for syntax in [
        "hydra status",
        "hydra repair",
        "hydra head list",
        "hydra head status <NAME>",
        "hydra head path <NAME>",
    ] {
        assert!(
            stdout.contains(syntax),
            "top-level help should expose {syntax:?}, got: {stdout:?}"
        );
    }
}

#[test]
fn head_help_exposes_every_available_inspection_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["head", "--help"])
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");
    for command in ["list", "status", "path"] {
        assert!(
            stdout.contains(command),
            "head help should list {command:?}, got: {stdout:?}"
        );
    }
}

#[test]
fn head_remove_help_documents_force_and_safe_default() {
    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["head", "remove", "--help"])
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");
    assert!(stdout.contains("Usage: hydra head remove [OPTIONS] <NAME>"));
    assert!(stdout.contains("--force"));
    assert!(stdout.contains("must be clean"));
    assert!(stdout.contains("integrated"));
    assert!(stdout.contains("hydra head remove payment --force"));
}

#[test]
fn head_close_help_documents_integration_and_protected_removal() {
    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["head", "close", "--help"])
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");
    assert!(stdout.contains("Usage: hydra head close <NAME>"));
    assert!(stdout.contains("must be clean"));
    assert!(stdout.contains("checked out in a clean worktree"));
    assert!(stdout.contains("checkout-free"));
    assert!(stdout.contains("strategy and result"));
    assert!(stdout.contains("protected removal"));
    assert!(stdout.contains("commands.close"));
    assert!(stdout.contains("removeOnSuccess"));
    assert!(stdout.contains("hydra head close payment"));
}

#[test]
fn repair_help_documents_reconciliation_and_confirmation() {
    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["repair", "--help"])
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");
    assert!(stdout.contains("Usage: hydra repair"));
    assert!(stdout.contains("Git worktrees"));
    assert!(stdout.contains("confirmation"));
    assert!(stdout.contains("hydra repair"));
}

#[test]
fn head_open_help_documents_the_configured_adapter() {
    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["head", "open", "--help"])
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");
    assert!(stdout.contains("Usage: hydra head open <NAME>"));
    assert!(stdout.contains("configured command"));
    assert!(stdout.contains("hydra head open payment"));
}

#[test]
fn doctor_storage_help_documents_the_real_volume_probe() {
    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["doctor", "storage", "--help"])
        .output()
        .expect("Hydra CLI should start");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");
    assert!(stdout.contains("Usage: hydra doctor storage"));
    assert!(stdout.contains("real"));
    assert!(stdout.contains("Heads volume"));
    assert!(stdout.contains("environment"));
    assert!(stdout.contains("filesystem"));
    assert!(stdout.contains("hydra doctor storage"));
}

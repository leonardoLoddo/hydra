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
    assert!(stdout.contains("--from <FROM>"));
    assert!(
        stdout.contains("HEAD"),
        "help should document the default base, got: {stdout:?}"
    );
    assert!(stdout.contains("--target <TARGET>"));
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

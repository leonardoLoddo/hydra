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

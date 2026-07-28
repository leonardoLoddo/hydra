use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let unique = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("hydra-cli-{label}-{}-{unique}", std::process::id()));
        fs::create_dir(&path).expect("test directory should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).expect("owned test directory should be removed");
    }
}

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
fn init_rejects_a_directory_outside_a_git_repository() {
    let directory = TestDirectory::new("not-git");

    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args([
            "init",
            directory.path().to_str().expect("path should be UTF-8"),
        ])
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("error output should be UTF-8");
    assert!(
        stderr.contains("not a Git repository"),
        "error should identify the invalid directory, got: {stderr:?}"
    );
    assert!(
        !directory.path().join(".hydra.json").exists(),
        "failed initialization must not create project configuration"
    );
}

#[test]
fn init_creates_project_configuration_heads_directory_and_local_state() {
    let directory = TestDirectory::new("success");
    let repository = directory.path().join("SampleProject");
    fs::create_dir(&repository).expect("repository directory should be created");

    let git = Command::new("git")
        .args(["init", "--quiet"])
        .arg(&repository)
        .status()
        .expect("Git should start");
    assert!(git.success(), "temporary Git repository should be created");

    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .arg("init")
        .arg(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "initialization should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let config: serde_json::Value = serde_json::from_slice(
        &fs::read(repository.join(".hydra.json")).expect("project configuration should be created"),
    )
    .expect("project configuration should be valid JSON");

    assert_eq!(config["version"], 1);
    assert_eq!(config["headsDirectory"], "../SampleProject.heads");
    assert_eq!(config["branchPrefix"], "hydra/");
    assert_eq!(config["storage"]["mode"], "auto");
    assert_eq!(config["overlay"]["copy"][0], "... .gitignore");
    assert!(
        config["projectId"]
            .as_str()
            .is_some_and(|project_id| !project_id.is_empty()),
        "project ID should be generated"
    );

    assert!(
        directory.path().join("SampleProject.heads").is_dir(),
        "default sibling Heads directory should be created"
    );

    let state: serde_json::Value = serde_json::from_slice(
        &fs::read(repository.join(".git/hydra/heads.json")).expect("local state should be created"),
    )
    .expect("local state should be valid JSON");
    assert_eq!(state["version"], 1);
    assert_eq!(state["heads"], serde_json::json!({}));
}

#[test]
fn init_refuses_to_reuse_a_preexisting_default_heads_directory() {
    let directory = TestDirectory::new("heads-exist");
    let repository = directory.path().join("SampleProject");
    let heads_directory = directory.path().join("SampleProject.heads");
    fs::create_dir(&repository).expect("repository directory should be created");
    fs::create_dir(&heads_directory).expect("preexisting Heads directory should be created");

    let git = Command::new("git")
        .args(["init", "--quiet"])
        .arg(&repository)
        .status()
        .expect("Git should start");
    assert!(git.success(), "temporary Git repository should be created");

    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .arg("init")
        .arg(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("already exists"),
        "error should explain the destination conflict"
    );
    assert!(
        !repository.join(".hydra.json").exists(),
        "destination conflict must not create project configuration"
    );
    assert!(
        !repository.join(".git/hydra/heads.json").exists(),
        "destination conflict must not create local state"
    );
}

#[test]
fn init_rolls_back_the_heads_directory_when_local_state_cannot_be_created() {
    let directory = TestDirectory::new("state-failure");
    let repository = directory.path().join("SampleProject");
    fs::create_dir(&repository).expect("repository directory should be created");

    let git = Command::new("git")
        .args(["init", "--quiet"])
        .arg(&repository)
        .status()
        .expect("Git should start");
    assert!(git.success(), "temporary Git repository should be created");

    fs::write(repository.join(".git/hydra"), b"state path conflict")
        .expect("conflicting state path should be created");

    let output = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .arg("init")
        .arg(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    assert!(
        !repository.join(".hydra.json").exists(),
        "state failure must not publish project configuration"
    );
    assert!(
        !directory.path().join("SampleProject.heads").exists(),
        "state failure must roll back the newly created Heads directory"
    );
}

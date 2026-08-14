use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub struct TestDirectory {
    directory: tempfile::TempDir,
}

impl TestDirectory {
    pub fn new(label: &str) -> Self {
        let directory = tempfile::Builder::new()
            .prefix(&format!("hydra-cli-{label}-"))
            .tempdir()
            .expect("test directory should be created");
        Self { directory }
    }

    pub fn path(&self) -> &Path {
        self.directory.path()
    }
}

pub fn hydra_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_hydra"))
}

#[allow(dead_code)]
pub fn run_git(repository: &Path, arguments: &[&str]) -> Output {
    Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(arguments)
        .output()
        .expect("Git should start")
}

#[allow(dead_code)]
pub fn create_initialized_project(directory: &TestDirectory) -> PathBuf {
    let repository = directory.path().join("SampleProject");
    fs::create_dir(&repository).expect("repository directory should be created");

    let output = Command::new("git")
        .args(["init", "--quiet", "--initial-branch=main"])
        .arg(&repository)
        .output()
        .expect("Git should start");
    assert!(output.status.success());

    fs::create_dir(repository.join("src")).expect("source directory should be created");
    fs::write(repository.join("src/app.txt"), b"base\n").expect("tracked file should be written");
    let output = run_git(&repository, &["add", "."]);
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
            "fixture",
        ],
    );
    assert!(output.status.success());

    let output = hydra_command()
        .arg("init")
        .arg(&repository)
        .output()
        .expect("Hydra CLI should start");
    assert!(
        output.status.success(),
        "Hydra init should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    repository
}

#[allow(dead_code)]
pub fn heads_directory(repository: &Path) -> PathBuf {
    let locator_path = repository.join(".git/hydra/project.json");
    let locator: serde_json::Value = serde_json::from_slice(
        &fs::read(&locator_path).expect("local project locator should be readable"),
    )
    .expect("local project locator should be valid JSON");
    PathBuf::from(
        locator["headsDirectory"]
            .as_str()
            .expect("locator should contain a Heads directory"),
    )
}

#[allow(dead_code)]
pub fn head_state_path(repository: &Path) -> PathBuf {
    heads_directory(repository).join(".hydra/heads.json")
}

#[allow(dead_code)]
pub fn head_state_lock_path(repository: &Path) -> PathBuf {
    heads_directory(repository).join(".hydra/heads.json.lock")
}

#[allow(dead_code)]
pub fn assert_no_head_creation_artifacts(repository: &Path, name: &str) {
    let heads_directory = heads_directory(repository);
    assert!(
        !heads_directory.join(name).exists(),
        "failed creation must not leave a Head directory"
    );
    let branch = format!("refs/heads/hydra/{name}");
    let output = run_git(repository, &["show-ref", "--verify", "--quiet", &branch]);
    assert!(
        !output.status.success(),
        "failed creation must not leave a branch"
    );
    assert!(
        !head_state_lock_path(repository).exists(),
        "failed creation must release the state lock"
    );
    assert!(
        !heads_directory
            .join(format!(".hydra/pending-{name}.json"))
            .exists(),
        "handled creation failure must remove its pending journal"
    );
}

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
pub fn assert_no_head_creation_artifacts(repository: &Path, name: &str) {
    let heads_directory = repository
        .parent()
        .expect("repository should have a parent")
        .join("SampleProject.heads");
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
        !repository.join(".git/hydra/heads.json.lock").exists(),
        "failed creation must release the state lock"
    );
}

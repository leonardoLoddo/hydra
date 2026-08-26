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
pub fn canonical_path(path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    let path = fs::canonicalize(path)?;
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};

        let mut components = path.components();
        let Some(Component::Prefix(prefix)) = components.next() else {
            return Ok(path);
        };
        let mut simplified = match prefix.kind() {
            Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:", char::from(drive))),
            Prefix::VerbatimUNC(server, share) => {
                let mut root = PathBuf::from(r"\\");
                root.push(server);
                root.push(share);
                root
            }
            _ => return Ok(path),
        };
        simplified.extend(components);
        Ok(simplified)
    }
    #[cfg(not(windows))]
    Ok(path)
}

#[allow(dead_code)]
pub fn canonical_parent_path(path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    let path = path.as_ref();
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no parent")
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no file name")
    })?;
    canonical_path(parent).map(|parent| parent.join(file_name))
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

    let output = run_git(&repository, &["config", "core.autocrlf", "false"]);
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
pub fn overlay_copy_on_write_is_supported(repository: &Path, source: &Path) -> bool {
    let probe = tempfile::Builder::new()
        .prefix(".hydra-overlay-test-probe-")
        .tempdir_in(heads_directory(repository))
        .expect("overlay capability probe directory should be created");
    let destination = probe.path().join("candidate");

    reflink_copy::reflink(source, destination).is_ok()
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

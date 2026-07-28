use std::{path::Path, process::Command};

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

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(unix)]
use std::{ffi::OsString, os::unix::ffi::OsStringExt};

use super::InitError;

pub(super) struct Repository {
    pub(super) root: PathBuf,
    pub(super) git_common_directory: PathBuf,
}

pub(super) fn discover_repository(path: &Path) -> Result<Repository, InitError> {
    let root = git_path(path, "--show-toplevel", "repository root")?;
    let git_common_directory = git_path(&root, "--git-common-dir", "Git common directory")?;

    Ok(Repository {
        root,
        git_common_directory,
    })
}

fn git_path(path: &Path, argument: &str, field: &'static str) -> Result<PathBuf, InitError> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(path)
        .args(["rev-parse", "--path-format=absolute", argument])
        .output()
        .map_err(InitError::GitUnavailable)?;

    if !output.status.success() {
        return Err(InitError::GitCommandFailed {
            operation: field,
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let value = output.stdout.strip_suffix(b"\n").unwrap_or(&output.stdout);
    let value = value.strip_suffix(b"\r").unwrap_or(value);
    if value.is_empty() {
        return Err(InitError::InvalidGitOutput(field));
    }

    #[cfg(unix)]
    {
        Ok(PathBuf::from(OsString::from_vec(value.to_vec())))
    }
    #[cfg(not(unix))]
    {
        let value = std::str::from_utf8(value).map_err(|_| InitError::InvalidGitOutput(field))?;
        Ok(PathBuf::from(value))
    }
}

pub(super) fn repository_name_as_str(repository_name: &OsStr) -> Result<&str, InitError> {
    repository_name
        .to_str()
        .ok_or_else(|| InitError::UnsupportedRepositoryName(PathBuf::from(repository_name)))
}

#[cfg(test)]
mod tests {
    use super::{InitError, repository_name_as_str};

    #[cfg(unix)]
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    #[cfg(unix)]
    #[test]
    fn repository_names_must_be_losslessly_representable_in_json() {
        let name = OsStr::from_bytes(b"project-\xff");

        let error = repository_name_as_str(name).expect_err("invalid UTF-8 must be rejected");

        assert!(matches!(error, InitError::UnsupportedRepositoryName(_)));
    }
}

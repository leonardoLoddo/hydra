use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use super::{
    HeadError,
    git::{self, Repository},
    inspection::validated_head_path,
    state::{HeadMetadata, StateSnapshot, discover_project_repository},
    validate_head_name,
};

#[derive(Debug)]
pub struct OpenedHead {
    pub name: String,
    pub path: PathBuf,
}

/// Starts the configured open adapter for a validated local Head.
///
/// # Errors
///
/// Returns [`HeadError`] when the Head or its worktree is inconsistent, no
/// opener is configured, a placeholder is unsupported, the process cannot be
/// started, or the adapter exits unsuccessfully.
pub fn open_head(source_path: &Path, name: &str) -> Result<OpenedHead, HeadError> {
    validate_head_name(name)?;
    let repository = discover_project_repository(source_path)?;
    let snapshot = StateSnapshot::load(&repository)?;
    let path = validated_head_path(&snapshot.heads_directory()?, name, snapshot.head(name)?)?;
    validate_worktree(&repository, name, snapshot.head(name)?, &path)?;
    let command = snapshot
        .open_command()
        .ok_or(HeadError::OpenCommandNotConfigured)?;
    let placeholders = placeholders(name, snapshot.head(name)?);
    let program = expand(command.program(), &placeholders)?;
    if program.is_empty() {
        return Err(HeadError::InvalidOpenCommand(
            "program must not be empty".to_owned(),
        ));
    }
    let args = command
        .args()
        .iter()
        .map(|argument| expand(argument, &placeholders))
        .collect::<Result<Vec<_>, _>>()?;
    let status = Command::new(&program)
        .args(args)
        .current_dir(&path)
        .status()
        .map_err(|source| HeadError::OpenCommandUnavailable {
            program: program.clone(),
            source,
        })?;
    if !status.success() {
        return Err(HeadError::OpenCommandFailed {
            program,
            status: status.code(),
        });
    }

    Ok(OpenedHead {
        name: name.to_owned(),
        path,
    })
}

fn validate_worktree(
    repository: &Repository,
    name: &str,
    metadata: &HeadMetadata,
    path: &Path,
) -> Result<(), HeadError> {
    match fs::symlink_metadata(path) {
        Ok(path_metadata) if path_metadata.is_dir() => {}
        Ok(_) => return Err(inconsistent(name, "worktree path is not a real directory")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(inconsistent(name, "worktree path is missing"));
        }
        Err(source) => {
            return Err(HeadError::FileSystem {
                action: "inspect Head worktree before opening",
                path: path.to_path_buf(),
                source,
            });
        }
    }
    let worktree = git::registered_worktrees(repository)?
        .into_iter()
        .find(|worktree| worktree.path == path)
        .ok_or_else(|| inconsistent(name, "worktree is not registered with Git"))?;
    if worktree.branch.as_deref() != Some(metadata.head_ref()) {
        return Err(inconsistent(
            name,
            "worktree branch does not match metadata",
        ));
    }
    Ok(())
}

fn placeholders<'a>(name: &'a str, metadata: &'a HeadMetadata) -> BTreeMap<&'static str, &'a str> {
    BTreeMap::from([
        ("{name}", name),
        ("{path}", metadata.worktree_path()),
        ("{headRef}", metadata.head_ref()),
        ("{baseRef}", metadata.base_ref()),
        ("{targetRef}", metadata.target_ref()),
    ])
}

fn expand(
    template: &str,
    placeholders: &BTreeMap<&'static str, &str>,
) -> Result<String, HeadError> {
    let mut remaining = template;
    let mut expanded = String::with_capacity(template.len());
    while let Some(open) = remaining.find('{') {
        let (literal, placeholder_and_rest) = remaining.split_at(open);
        if literal.contains('}') {
            return Err(unsupported_placeholder(template));
        }
        expanded.push_str(literal);
        let Some(close) = placeholder_and_rest.find('}') else {
            return Err(unsupported_placeholder(template));
        };
        let (placeholder, rest) = placeholder_and_rest.split_at(close + 1);
        let value = placeholders
            .get(placeholder)
            .ok_or_else(|| unsupported_placeholder(template))?;
        expanded.push_str(value);
        remaining = rest;
    }
    if remaining.contains('}') {
        return Err(unsupported_placeholder(template));
    }
    expanded.push_str(remaining);
    if expanded.contains('\0') {
        return Err(HeadError::InvalidOpenCommand(
            "program and arguments must not contain NUL".to_owned(),
        ));
    }
    Ok(expanded)
}

fn unsupported_placeholder(template: &str) -> HeadError {
    HeadError::InvalidOpenCommand(format!("unsupported placeholder in {template:?}"))
}

fn inconsistent(name: &str, reason: &'static str) -> HeadError {
    HeadError::HeadOpenInconsistent {
        name: name.to_owned(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::expand;

    #[test]
    fn placeholder_values_may_contain_literal_braces() {
        let placeholders = BTreeMap::from([("{path}", "/projects/{demo}/payment")]);

        let expanded =
            expand("--folder={path}", &placeholders).expect("value braces should remain literal");

        assert_eq!(expanded, "--folder=/projects/{demo}/payment");
    }

    #[test]
    fn unsupported_template_placeholders_are_rejected() {
        let placeholders = BTreeMap::from([("{path}", "/projects/demo/payment")]);

        let error = expand("{unknown}", &placeholders)
            .expect_err("unknown placeholders must not reach the adapter");

        assert!(error.to_string().contains("unsupported placeholder"));
    }
}

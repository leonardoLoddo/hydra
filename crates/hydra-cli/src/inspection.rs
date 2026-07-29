use std::{
    io::{self, IsTerminal, Write},
    path::Path,
    process::ExitCode,
};

use crate::output::{safe_path_label, safe_terminal_text};

pub(super) fn show_project_status() -> ExitCode {
    match hydra_core::inspect_project(Path::new(".")) {
        Ok(project) => {
            println!("Project: {}", safe_path_label(&project.repository_root));
            println!(
                "Heads directory: {}",
                safe_path_label(&project.heads_directory)
            );
            println!("Heads: {}", project.heads.len());
            for head in project.heads {
                println!("  {}  {}", head.name, head.status);
            }
            ExitCode::SUCCESS
        }
        Err(error) => fail(&error),
    }
}

pub(super) fn list_heads() -> ExitCode {
    match hydra_core::list_heads(Path::new(".")) {
        Ok(heads) => {
            for name in heads {
                println!("{name}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => fail(&error),
    }
}

pub(super) fn show_head_status(name: &str) -> ExitCode {
    match hydra_core::inspect_head(Path::new("."), name) {
        Ok(head) => {
            println!("Head: {}", head.name);
            println!("Path: {}", safe_path_label(&head.path));
            match &head.worktree_head {
                hydra_core::WorktreeHead::Branch(reference)
                    if reference == &head.recorded_head_ref =>
                {
                    println!("Branch: {}", safe_terminal_text(reference));
                }
                hydra_core::WorktreeHead::Branch(reference) => {
                    println!(
                        "Branch: {} (expected {})",
                        safe_terminal_text(reference),
                        safe_terminal_text(&head.recorded_head_ref)
                    );
                }
                hydra_core::WorktreeHead::Detached => {
                    println!(
                        "Branch: detached (expected {})",
                        safe_terminal_text(&head.recorded_head_ref)
                    );
                }
                hydra_core::WorktreeHead::Unavailable => {
                    println!(
                        "Branch: unavailable (expected {})",
                        safe_terminal_text(&head.recorded_head_ref)
                    );
                }
            }
            println!(
                "Commit: {}",
                head.commit.as_deref().unwrap_or("unavailable")
            );
            println!(
                "Base: {} ({})",
                safe_terminal_text(&head.base_ref),
                safe_terminal_text(&head.base_commit)
            );
            println!("Target: {}", safe_terminal_text(&head.target_ref));
            if let Some(changes) = head.changes {
                println!(
                    "Changes: {} modified, {} added, {} deleted, {} untracked",
                    changes.modified, changes.added, changes.deleted, changes.untracked
                );
            } else {
                println!("Changes: unavailable");
            }
            match (head.ahead, head.behind) {
                (Some(ahead), Some(behind)) => println!("Ahead/behind: {ahead}/{behind}"),
                _ => println!("Ahead/behind: unavailable"),
            }
            println!(
                "Worktree: {}",
                if head.worktree_present {
                    "present"
                } else {
                    "missing"
                }
            );
            if head.consistency_issues.is_empty() {
                println!("Consistency: ok");
            } else {
                println!("Consistency: {}", head.consistency_issues.join("; "));
            }
            ExitCode::SUCCESS
        }
        Err(error) => fail(&error),
    }
}

pub(super) fn show_head_path(name: &str) -> ExitCode {
    match hydra_core::head_path(Path::new("."), name) {
        Ok(path) => {
            let stdout = io::stdout();
            let terminal = stdout.is_terminal();
            match write_head_path(&mut stdout.lock(), &path, terminal) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("error: failed to show the Head path: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(error) => fail(&error),
    }
}

fn write_head_path(output: &mut impl Write, path: &Path, terminal: bool) -> io::Result<()> {
    if terminal {
        write!(output, "{}", safe_path_label(path))?;
    } else {
        write_raw_path(output, path)?;
    }
    writeln!(output)
}

#[cfg(unix)]
fn write_raw_path(output: &mut impl Write, path: &Path) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    output.write_all(path.as_os_str().as_bytes())
}

#[cfg(not(unix))]
fn write_raw_path(output: &mut impl Write, path: &Path) -> io::Result<()> {
    write!(output, "{}", path.display())
}

fn fail(error: &hydra_core::HeadError) -> ExitCode {
    eprintln!("error: {error}");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::write_head_path;

    #[test]
    fn head_path_escapes_control_characters_on_a_terminal() {
        let mut output = Vec::new();

        write_head_path(
            &mut output,
            Path::new("/projects/\u{1b}demo\npayment"),
            true,
        )
        .expect("path should be written");

        assert_eq!(
            String::from_utf8(output).expect("output should be UTF-8"),
            "/projects/\\u{1b}demo\\npayment\n"
        );
    }

    #[test]
    fn head_path_preserves_control_characters_for_a_pipeline() {
        let mut output = Vec::new();

        write_head_path(
            &mut output,
            Path::new("/projects/\u{1b}demo\npayment"),
            false,
        )
        .expect("path should be written");

        assert_eq!(output, b"/projects/\x1bdemo\npayment\n");
    }
}

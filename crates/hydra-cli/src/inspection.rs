use std::{
    io::{self, IsTerminal, Write},
    path::Path,
    process::ExitCode,
};

use serde::Serialize;

use crate::output::{safe_path_label, safe_terminal_text};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectJson<'a> {
    schema_version: u32,
    repository_root: &'a str,
    heads_directory: &'a str,
    head_count: usize,
    heads: Vec<HeadSummaryJson<'a>>,
}

#[derive(Serialize)]
struct HeadSummaryJson<'a> {
    name: &'a str,
    status: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HeadListJson<'a> {
    schema_version: u32,
    heads: &'a [String],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HeadPathJson<'a> {
    schema_version: u32,
    name: &'a str,
    path: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HeadStatusJson<'a> {
    schema_version: u32,
    name: &'a str,
    recorded: RecordedHeadJson<'a>,
    observed: ObservedHeadJson<'a>,
    consistency: ConsistencyJson<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordedHeadJson<'a> {
    path: &'a str,
    head_ref: &'a str,
    base_ref: &'a str,
    base_commit: &'a str,
    target_ref: &'a str,
    materialization_backend: &'a str,
    created_at: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ObservedHeadJson<'a> {
    worktree_head: WorktreeHeadJson<'a>,
    commit: Option<&'a str>,
    changes: Option<ChangeCountsJson>,
    ahead: Option<usize>,
    behind: Option<usize>,
    worktree_present: bool,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum WorktreeHeadJson<'a> {
    Branch { reference: &'a str },
    Detached,
    Unavailable,
}

#[derive(Serialize)]
struct ChangeCountsJson {
    modified: usize,
    added: usize,
    deleted: usize,
    untracked: usize,
}

#[derive(Serialize)]
struct ConsistencyJson<'a> {
    status: &'static str,
    issues: &'a [String],
}

pub(super) fn show_project_status(json: bool) -> ExitCode {
    match hydra_core::inspect_project(Path::new(".")) {
        Ok(project) => {
            if json {
                return show_project_json(&project);
            }
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

pub(super) fn list_heads(json: bool) -> ExitCode {
    match hydra_core::list_heads(Path::new(".")) {
        Ok(heads) => {
            if json {
                return write_json(&HeadListJson {
                    schema_version: 1,
                    heads: &heads,
                });
            }
            for name in heads {
                println!("{name}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => fail(&error),
    }
}

pub(super) fn show_head_status(name: &str, json: bool) -> ExitCode {
    match hydra_core::inspect_head(Path::new("."), name) {
        Ok(head) => {
            if json {
                return show_head_json(&head);
            }
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

fn show_project_json(project: &hydra_core::ProjectInspection) -> ExitCode {
    let repository_root = match crate::json_output::path(&project.repository_root) {
        Ok(path) => path,
        Err(error) => return fail_json(&error),
    };
    let heads_directory = match crate::json_output::path(&project.heads_directory) {
        Ok(path) => path,
        Err(error) => return fail_json(&error),
    };
    let heads = project
        .heads
        .iter()
        .map(|head| HeadSummaryJson {
            name: &head.name,
            status: head.status,
        })
        .collect();
    write_json(&ProjectJson {
        schema_version: 1,
        repository_root,
        heads_directory,
        head_count: project.heads.len(),
        heads,
    })
}

fn show_head_json(head: &hydra_core::HeadInspection) -> ExitCode {
    let path = match crate::json_output::path(&head.path) {
        Ok(path) => path,
        Err(error) => return fail_json(&error),
    };
    let worktree_head = match &head.worktree_head {
        hydra_core::WorktreeHead::Branch(reference) => WorktreeHeadJson::Branch { reference },
        hydra_core::WorktreeHead::Detached => WorktreeHeadJson::Detached,
        hydra_core::WorktreeHead::Unavailable => WorktreeHeadJson::Unavailable,
    };
    let changes = head.changes.as_ref().map(|changes| ChangeCountsJson {
        modified: changes.modified,
        added: changes.added,
        deleted: changes.deleted,
        untracked: changes.untracked,
    });
    write_json(&HeadStatusJson {
        schema_version: 1,
        name: &head.name,
        recorded: RecordedHeadJson {
            path,
            head_ref: &head.recorded_head_ref,
            base_ref: &head.base_ref,
            base_commit: &head.base_commit,
            target_ref: &head.target_ref,
            materialization_backend: &head.materialization_backend,
            created_at: &head.created_at,
        },
        observed: ObservedHeadJson {
            worktree_head,
            commit: head.commit.as_deref(),
            changes,
            ahead: head.ahead,
            behind: head.behind,
            worktree_present: head.worktree_present,
        },
        consistency: ConsistencyJson {
            status: if head.consistency_issues.is_empty() {
                "ok"
            } else {
                "inconsistent"
            },
            issues: &head.consistency_issues,
        },
    })
}

fn write_json(value: &impl Serialize) -> ExitCode {
    match crate::json_output::write(value) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail_json(&error),
    }
}

fn fail_json(error: &str) -> ExitCode {
    eprintln!("error: {error}");
    eprintln!("next: Fix the reported JSON output problem and rerun the command.");
    ExitCode::FAILURE
}

pub(super) fn show_head_path(name: &str, json: bool) -> ExitCode {
    match hydra_core::head_path(Path::new("."), name) {
        Ok(path) => {
            if json {
                let path = match crate::json_output::path(&path) {
                    Ok(path) => path,
                    Err(error) => return fail_json(&error),
                };
                return write_json(&HeadPathJson {
                    schema_version: 1,
                    name,
                    path,
                });
            }
            let stdout = io::stdout();
            let terminal = stdout.is_terminal();
            match write_head_path(&mut stdout.lock(), &path, terminal) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("error: failed to show the Head path: {error}");
                    eprintln!("next: Check the stdout destination and rerun `hydra head path`.");
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
    crate::guidance::report_head_error(error);
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

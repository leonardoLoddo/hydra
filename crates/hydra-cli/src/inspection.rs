use std::{path::Path, process::ExitCode};

pub(super) fn show_project_status() -> ExitCode {
    match hydra_core::inspect_project(Path::new(".")) {
        Ok(project) => {
            println!("Project: {}", project.repository_root.display());
            println!("Heads directory: {}", project.heads_directory.display());
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
            println!("Path: {}", head.path.display());
            println!("Branch: {}", head.head_ref);
            println!(
                "Commit: {}",
                head.commit.as_deref().unwrap_or("unavailable")
            );
            println!("Base: {} ({})", head.base_ref, head.base_commit);
            println!("Target: {}", head.target_ref);
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
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => fail(&error),
    }
}

fn fail(error: &hydra_core::HeadError) -> ExitCode {
    eprintln!("error: {error}");
    ExitCode::FAILURE
}

use std::{
    io::{self, BufRead, IsTerminal, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use crate::output::safe_path_label;

pub(super) fn run(name: &str, from: Option<&str>, target: Option<&str>) -> ExitCode {
    let progress_enabled = io::stderr().is_terminal();
    let create = |confirmed_full_copy, exclude_unsafe_overlay_symlinks| {
        hydra_core::create_head_with_progress(
            Path::new("."),
            hydra_core::CreateHeadOptions {
                name: name.to_owned(),
                from: from.map(str::to_owned),
                target: target.map(str::to_owned),
                confirmed_full_copy,
                exclude_unsafe_overlay_symlinks,
            },
            |progress| {
                if progress_enabled {
                    show_progress(progress);
                }
            },
        )
    };

    let mut exclude_unsafe_overlay_symlinks = false;
    let first_result = create(false, false);
    let result = match first_result {
        Err(hydra_core::HeadError::UnsafeOverlaySymlinks { paths }) => {
            let stdin = io::stdin();
            let mut input = stdin.lock();
            let stdout = io::stdout();
            let mut output = stdout.lock();
            let confirmed =
                request_unsafe_symlink_exclusion(&mut input, &mut output, paths.as_slice());
            let confirmed = match confirmed {
                Ok(confirmed) => confirmed,
                Err(error) => {
                    eprintln!(
                        "error: failed to read unsafe-symlink exclusion confirmation: {error}"
                    );
                    return ExitCode::FAILURE;
                }
            };
            if !confirmed {
                eprintln!("error: Head creation cancelled");
                return ExitCode::FAILURE;
            }
            exclude_unsafe_overlay_symlinks = true;
            create(false, true)
        }
        result => result,
    };

    match result {
        Err(hydra_core::HeadError::OverlayFullCopyConfirmationRequired { files, bytes }) => {
            let stdin = io::stdin();
            let mut input = stdin.lock();
            let stdout = io::stdout();
            let mut output = stdout.lock();
            let confirmed = request_full_copy_confirmation(&mut input, &mut output, files, bytes);
            let confirmed = match confirmed {
                Ok(confirmed) => confirmed,
                Err(error) => {
                    eprintln!("error: failed to read full-copy confirmation: {error}");
                    return ExitCode::FAILURE;
                }
            };
            if !confirmed {
                eprintln!("error: Head creation cancelled");
                return ExitCode::FAILURE;
            }
            finish(create(true, exclude_unsafe_overlay_symlinks))
        }
        result => finish(result),
    }
}

fn show_progress(progress: hydra_core::HeadCreationProgress) {
    let stderr = io::stderr();
    let _ = write_progress(&mut stderr.lock(), progress);
}

fn write_progress(
    output: &mut impl Write,
    progress: hydra_core::HeadCreationProgress,
) -> io::Result<()> {
    match progress {
        hydra_core::HeadCreationProgress::PlanningOverlays => {
            writeln!(output, "Planning overlays...")
        }
        hydra_core::HeadCreationProgress::MaterializingTrackedEntries { entries } => {
            writeln!(output, "Materializing {entries} tracked entries...")
        }
        hydra_core::HeadCreationProgress::MaterializingOverlayEntries { entries } => {
            writeln!(output, "Materializing {entries} overlay entries...")
        }
        _ => Ok(()),
    }
}

fn finish(result: Result<hydra_core::CreatedHead, hydra_core::HeadError>) -> ExitCode {
    match result {
        Ok(head) => {
            if head.overlay_files > 0 {
                println!(
                    "Overlay: {} file(s), {} byte(s)",
                    head.overlay_files, head.overlay_bytes
                );
            }
            let stdout = io::stdout();
            let hyperlinks_enabled = stdout.is_terminal();
            if let Err(error) =
                write_created_head_path(&mut stdout.lock(), &head.path, hyperlinks_enabled)
            {
                eprintln!("error: failed to show the created Head path: {error}");
                return ExitCode::FAILURE;
            }
            match head.storage_backend {
                hydra_core::StorageBackend::CopyOnWrite => {
                    println!("Storage backend: copy-on-write");
                }
                hydra_core::StorageBackend::FullCopy => {
                    println!("Storage backend: full copy");
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn request_full_copy_confirmation(
    input: &mut impl BufRead,
    output: &mut impl Write,
    files: usize,
    bytes: u64,
) -> io::Result<bool> {
    write!(
        output,
        "Full copy required: {files} file(s), {bytes} byte(s)\nContinue? [y/N] "
    )?;
    output.flush()?;

    let mut response = String::new();
    input.read_line(&mut response)?;
    Ok(matches!(
        response.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn request_unsafe_symlink_exclusion(
    input: &mut impl BufRead,
    output: &mut impl Write,
    paths: &[PathBuf],
) -> io::Result<bool> {
    writeln!(output, "Unsafe overlay symlinks:")?;
    for path in paths {
        writeln!(output, "  {}", safe_path_label(path))?;
    }
    write!(output, "Exclude them and update .hydra.json? [y/N] ")?;
    output.flush()?;

    let mut response = String::new();
    input.read_line(&mut response)?;
    Ok(matches!(
        response.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn write_created_head_path(
    output: &mut impl Write,
    path: &Path,
    hyperlinks_enabled: bool,
) -> io::Result<()> {
    let label = safe_path_label(path);
    write!(output, "New Head successfully created at ")?;
    if hyperlinks_enabled && let Some(uri) = file_uri(path) {
        write!(output, "\u{1b}]8;;{uri}\u{1b}\\{label}\u{1b}]8;;\u{1b}\\")?;
    } else {
        write!(output, "{label}")?;
    }
    writeln!(output)
}

#[cfg(unix)]
fn file_uri(path: &Path) -> Option<String> {
    use std::os::unix::ffi::OsStrExt;

    if !path.is_absolute() {
        return None;
    }
    let mut uri = String::from("file://");
    for byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            uri.push(char::from(*byte));
        } else {
            use std::fmt::Write as _;
            write!(uri, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    Some(uri)
}

#[cfg(not(unix))]
fn file_uri(_path: &Path) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, path::Path};

    use super::{request_full_copy_confirmation, write_created_head_path, write_progress};

    #[test]
    fn full_copy_confirmation_is_explicit_and_defaults_to_no() {
        let mut input = Cursor::new(b"\n");
        let mut output = Vec::new();

        let confirmed = request_full_copy_confirmation(&mut input, &mut output, 2, 13)
            .expect("prompt should be written and read");

        assert!(!confirmed);
        assert_eq!(
            String::from_utf8(output).expect("prompt should be UTF-8"),
            "Full copy required: 2 file(s), 13 byte(s)\nContinue? [y/N] "
        );
    }

    #[test]
    fn interactive_progress_describes_the_current_phase() {
        let mut output = Vec::new();

        write_progress(
            &mut output,
            hydra_core::HeadCreationProgress::MaterializingTrackedEntries { entries: 1_840 },
        )
        .expect("tracked progress should be written");
        write_progress(
            &mut output,
            hydra_core::HeadCreationProgress::MaterializingOverlayEntries { entries: 2_000 },
        )
        .expect("progress should be written");

        assert_eq!(
            String::from_utf8(output).expect("progress should be UTF-8"),
            concat!(
                "Materializing 1840 tracked entries...\n",
                "Materializing 2000 overlay entries...\n"
            )
        );
    }

    #[test]
    fn created_head_path_uses_an_osc_8_file_link_when_enabled() {
        let mut output = Vec::new();

        write_created_head_path(
            &mut output,
            Path::new("/projects/Hydra Demo.heads/payment#retry"),
            true,
        )
        .expect("success message should be written");

        assert_eq!(
            String::from_utf8(output).expect("message should be UTF-8"),
            "New Head successfully created at \u{1b}]8;;file:///projects/Hydra%20Demo.heads/payment%23retry\u{1b}\\/projects/Hydra Demo.heads/payment#retry\u{1b}]8;;\u{1b}\\\n"
        );
    }

    #[test]
    fn created_head_path_remains_plain_when_hyperlinks_are_disabled() {
        let mut output = Vec::new();

        write_created_head_path(
            &mut output,
            Path::new("/projects/demo.heads/payment"),
            false,
        )
        .expect("success message should be written");

        assert_eq!(
            String::from_utf8(output).expect("message should be UTF-8"),
            "New Head successfully created at /projects/demo.heads/payment\n"
        );
    }

    #[test]
    fn created_head_path_encodes_control_characters_in_uri_and_visible_text() {
        let mut output = Vec::new();

        write_created_head_path(
            &mut output,
            Path::new("/projects/demo\nheads/payment"),
            true,
        )
        .expect("success message should be written");

        assert_eq!(
            String::from_utf8(output).expect("message should be UTF-8"),
            "New Head successfully created at \u{1b}]8;;file:///projects/demo%0Aheads/payment\u{1b}\\/projects/demo\\nheads/payment\u{1b}]8;;\u{1b}\\\n"
        );
    }
}

mod common;

use std::{fs, io::Write, process::Stdio, time::Instant};

use common::{TestDirectory, create_initialized_project, hydra_command, run_git};

#[test]
#[ignore = "performance benchmark; run explicitly with --release --ignored --nocapture"]
fn head_creation_with_a_large_overlay_reports_elapsed_time() {
    let file_count = std::env::var("HYDRA_BENCHMARK_FILES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10_000);
    let directory = TestDirectory::new("head-create-performance");
    let repository = create_initialized_project(&directory);
    fs::write(repository.join(".gitignore"), b"dependencies/\n")
        .expect("overlay rules should be written");
    let output = run_git(&repository, &["add", ".gitignore"]);
    assert!(output.status.success());
    let output = run_git(
        &repository,
        &[
            "-c",
            "user.name=Hydra Benchmark",
            "-c",
            "user.email=hydra-benchmark@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "add benchmark overlay rules",
        ],
    );
    assert!(output.status.success());

    let dependencies = repository.join("dependencies");
    fs::create_dir(&dependencies).expect("dependency directory should be created");
    for index in 0..file_count {
        fs::write(
            dependencies.join(format!("dependency-{index}.txt")),
            format!("dependency-{index}\n"),
        )
        .expect("dependency fixture should be written");
    }

    let started = Instant::now();
    let mut child = hydra_command()
        .args(["head", "create", "benchmark"])
        .current_dir(&repository)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Hydra CLI should start");
    child
        .stdin
        .take()
        .expect("benchmark stdin should be piped")
        .write_all(b"yes\n")
        .expect("full-copy confirmation should be writable");
    let output = child
        .wait_with_output()
        .expect("Hydra benchmark should complete");
    let elapsed = started.elapsed();

    assert!(
        output.status.success(),
        "benchmark Head creation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!(
        "created a Head with {file_count} overlay files in {:.3} seconds",
        elapsed.as_secs_f64()
    );
}

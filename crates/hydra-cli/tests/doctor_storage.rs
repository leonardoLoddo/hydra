mod common;

use std::{ffi::OsString, fs, path::Path};

use common::{
    TestDirectory, copy_on_write_guidance_url, create_initialized_project, head_state_lock_path,
    heads_directory, hydra_command, run_git,
};

fn directory_entries(path: &Path) -> Vec<OsString> {
    let mut entries: Vec<OsString> = fs::read_dir(path)
        .expect("directory should be readable")
        .map(|entry| {
            entry
                .expect("directory entry should be readable")
                .file_name()
        })
        .collect();
    entries.sort();
    entries
}

#[test]
fn doctor_storage_runs_a_real_probe_and_cleans_every_artifact() {
    let directory = TestDirectory::new("doctor-storage");
    let repository = create_initialized_project(&directory);
    let heads = heads_directory(&repository);
    let entries_before = directory_entries(&heads);

    let output = hydra_command()
        .args(["doctor", "storage"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "storage diagnostics should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(
        stdout.contains("Storage backend: copy-on-write\n")
            || stdout.contains("Storage backend: full copy\n")
    );
    assert!(stdout.contains("Native primitive: "));
    assert!(stdout.contains("Environment: "));
    assert!(stdout.contains("Filesystem: "));
    assert!(stdout.contains("Fallback: full copy (verified)\n"));
    assert!(stdout.contains("Mutable hard links: disabled\n"));
    assert!(stdout.contains("Isolation: supported\n"));
    if let Some(guidance) = copy_on_write_guidance_url() {
        let guidance = format!("Copy-on-write guidance: {guidance}\n");
        if stdout.contains("Storage backend: full copy\n") {
            assert!(stdout.contains(&guidance));
        } else {
            assert!(!stdout.contains(&guidance));
        }
    }
    assert_eq!(directory_entries(&heads), entries_before);
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn doctor_storage_emits_versioned_json_and_cleans_every_artifact() {
    let directory = TestDirectory::new("doctor-storage-json");
    let repository = create_initialized_project(&directory);
    let heads = heads_directory(&repository);
    let entries_before = directory_entries(&heads);

    let output = hydra_command()
        .args(["doctor", "storage", "--json"])
        .current_dir(&repository)
        .output()
        .expect("Hydra CLI should start");

    assert!(
        output.status.success(),
        "JSON storage diagnostics should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("storage diagnostics should be valid JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert!(matches!(
        json["storageBackend"].as_str(),
        Some("copyOnWrite" | "fullCopy")
    ));
    assert!(matches!(
        json["nativePrimitive"].as_str(),
        Some(
            "apfsClone" | "linuxReflink" | "windowsReFsBlockClone" | "nativeClone" | "unavailable"
        )
    ));
    assert!(matches!(
        json["environment"].as_str(),
        Some("native" | "windowsSubsystemForLinux")
    ));
    assert!(json["filesystem"].is_null() || json["filesystem"].is_string());
    assert!(json["copyOnWriteGuidance"].is_null() || json["copyOnWriteGuidance"].is_string());
    assert_eq!(json["fullCopyFallbackVerified"], true);
    assert_eq!(json["mutableHardLinksEnabled"], false);
    assert_eq!(json["isolationSupported"], true);
    assert_eq!(
        json.as_object().expect("report should be an object").len(),
        9
    );
    let (terminator, content) = output
        .stdout
        .split_last()
        .expect("JSON output should contain a value and a newline");
    assert_eq!(*terminator, b'\n');
    assert!(!content.contains(&b'\n'));
    assert_eq!(directory_entries(&heads), entries_before);
    assert!(!head_state_lock_path(&repository).exists());
}

#[test]
fn doctor_storage_requires_an_initialized_hydra_project() {
    let directory = TestDirectory::new("doctor-storage-uninitialized");
    let initialized = run_git(directory.path(), &["init", "--quiet"]);
    assert!(initialized.status.success());

    let output = hydra_command()
        .args(["doctor", "storage"])
        .current_dir(directory.path())
        .output()
        .expect("Hydra CLI should start");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not initialized"));
    assert!(stderr.contains("next: Run `hydra init [PATH]`"));
    assert!(!directory.path().join(".hydra.json").exists());
}

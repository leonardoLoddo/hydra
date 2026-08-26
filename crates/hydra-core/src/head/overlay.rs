mod hash;
mod materialization;

pub(super) use materialization::materialize_overlays;

use std::{
    collections::{BTreeSet, HashSet},
    fs, io,
    path::{Component, Path, PathBuf},
};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use uuid::Uuid;

use super::{HeadError, git::TrackedEntry};

#[derive(Debug)]
struct OverlayFile {
    source: PathBuf,
    relative: PathBuf,
    size: u64,
    kind: OverlayKind,
}

struct CopyOnWriteProbe {
    directory: PathBuf,
    destination: PathBuf,
    active: bool,
}

impl CopyOnWriteProbe {
    fn create(heads_directory: &Path) -> Result<Self, HeadError> {
        let directory =
            heads_directory.join(format!(".hydra-overlay-probe-{}", Uuid::new_v4().simple()));
        fs::create_dir(&directory).map_err(|source| HeadError::FileSystem {
            action: "create isolated overlay probe directory",
            path: directory.clone(),
            source,
        })?;
        Ok(Self {
            destination: directory.join("candidate"),
            directory,
            active: true,
        })
    }

    fn assess(&mut self, source: &Path) -> Result<bool, HeadError> {
        let cloned = reflink_copy::reflink(source, &self.destination).is_ok();
        match fs::remove_file(&self.destination) {
            Ok(()) => Ok(cloned),
            Err(error) if error.kind() == io::ErrorKind::NotFound && !cloned => Ok(false),
            Err(source) => Err(HeadError::FileSystem {
                action: "remove temporary overlay probe",
                path: self.destination.clone(),
                source,
            }),
        }
    }

    fn finish(&mut self) -> Result<(), HeadError> {
        fs::remove_dir(&self.directory).map_err(|source| HeadError::FileSystem {
            action: "remove isolated overlay probe directory",
            path: self.directory.clone(),
            source,
        })?;
        self.active = false;
        Ok(())
    }
}

impl Drop for CopyOnWriteProbe {
    fn drop(&mut self) {
        if self.active {
            let _ = fs::remove_file(&self.destination);
            let _ = fs::remove_dir(&self.directory);
        }
    }
}

#[derive(Debug)]
enum OverlayKind {
    Regular {
        identity: Option<String>,
        requires_full_copy: bool,
    },
    Symlink {
        target: PathBuf,
    },
}

impl OverlayFile {
    fn requires_full_copy(&self) -> bool {
        matches!(
            self.kind,
            OverlayKind::Regular {
                requires_full_copy: true,
                ..
            }
        )
    }
}

#[derive(Debug)]
pub(super) struct OverlayPlan {
    files: Vec<OverlayFile>,
    total_bytes: u64,
}

impl OverlayPlan {
    pub(super) fn file_count(&self) -> usize {
        self.files.len()
    }

    pub(super) fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub(super) fn full_copy_file_count(&self) -> usize {
        self.files
            .iter()
            .filter(|file| file.requires_full_copy())
            .count()
    }

    pub(super) fn full_copy_bytes(&self) -> u64 {
        self.files
            .iter()
            .filter(|file| file.requires_full_copy())
            .map(|file| file.size)
            .sum()
    }
}

pub(super) fn plan_overlays(
    source_root: &Path,
    heads_directory: &Path,
    rules: &[String],
    tracked_entries: &[TrackedEntry],
    force_full_copy: bool,
) -> Result<OverlayPlan, HeadError> {
    let canonical_source_root =
        crate::path::canonicalize(source_root).map_err(|source| HeadError::FileSystem {
            action: "resolve overlay source root",
            path: source_root.to_path_buf(),
            source,
        })?;
    let matcher = build_matcher(source_root, rules)?;
    let tracked = tracked_entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<HashSet<_>>();
    let mut files = Vec::new();
    let mut unsafe_symlinks = BTreeSet::new();
    visit_directory(
        source_root,
        &canonical_source_root,
        source_root,
        &matcher,
        &tracked,
        &mut files,
        &mut unsafe_symlinks,
    )?;
    if !unsafe_symlinks.is_empty() {
        return Err(HeadError::UnsafeOverlaySymlinks {
            paths: unsafe_symlinks.into_iter().collect(),
        });
    }
    files.sort_by(|left, right| left.relative.cmp(&right.relative));
    assign_regular_identities(source_root, &mut files)?;
    if force_full_copy {
        classify_copy_fallbacks_with(&mut files, |_| Ok(false))?;
    } else {
        let mut probe = CopyOnWriteProbe::create(heads_directory)?;
        let assessment = classify_copy_fallbacks_with(&mut files, |source| probe.assess(source));
        let cleanup = probe.finish();
        match (assessment, cleanup) {
            (Ok(()), Ok(())) => {}
            (Err(error), Ok(())) | (Ok(()), Err(error)) => return Err(error),
            (Err(original), Err(cleanup)) => {
                return Err(HeadError::RollbackFailed {
                    original: Box::new(original),
                    failures: vec![cleanup.to_string()],
                });
            }
        }
    }
    let total_bytes = files.iter().try_fold(0_u64, |total, file| {
        total
            .checked_add(file.size)
            .ok_or_else(|| HeadError::OverlayRules("overlay size overflow".to_owned()))
    })?;
    Ok(OverlayPlan { files, total_bytes })
}

fn build_matcher(source_root: &Path, rules: &[String]) -> Result<Gitignore, HeadError> {
    let mut builder = GitignoreBuilder::new(source_root);
    for rule in rules {
        if let Some(included) = rule.strip_prefix("... ") {
            let relative = Path::new(included);
            validate_relative_path(relative)?;
            let path = source_root.join(relative);
            match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_file() => {
                    if let Some(error) = builder.add(&path) {
                        return Err(HeadError::OverlayRules(error.to_string()));
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Ok(_) => return Err(HeadError::UnsafeOverlayPath(path)),
                Err(source) => {
                    return Err(HeadError::FileSystem {
                        action: "inspect included overlay rules",
                        path,
                        source,
                    });
                }
            }
        } else {
            builder
                .add_line(None, rule)
                .map_err(|error| HeadError::OverlayRules(error.to_string()))?;
        }
    }
    builder
        .build()
        .map_err(|error| HeadError::OverlayRules(error.to_string()))
}

fn visit_directory(
    source_root: &Path,
    canonical_source_root: &Path,
    directory: &Path,
    matcher: &Gitignore,
    tracked: &HashSet<PathBuf>,
    files: &mut Vec<OverlayFile>,
    unsafe_symlinks: &mut BTreeSet<PathBuf>,
) -> Result<(), HeadError> {
    let mut entries = fs::read_dir(directory)
        .map_err(|source| HeadError::FileSystem {
            action: "scan overlay source directory",
            path: directory.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| HeadError::FileSystem {
            action: "read overlay source entry",
            path: directory.to_path_buf(),
            source,
        })?;
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let source = entry.path();
        let relative = source
            .strip_prefix(source_root)
            .map_err(|_| HeadError::UnsafeOverlayPath(source.clone()))?
            .to_path_buf();
        validate_relative_path(&relative)?;
        if relative
            .components()
            .next()
            .is_some_and(|component| component.as_os_str() == ".git")
        {
            continue;
        }

        let metadata =
            fs::symlink_metadata(&source).map_err(|source_error| HeadError::FileSystem {
                action: "inspect overlay source",
                path: source.clone(),
                source: source_error,
            })?;
        if metadata.is_dir() {
            visit_directory(
                source_root,
                canonical_source_root,
                &source,
                matcher,
                tracked,
                files,
                unsafe_symlinks,
            )?;
        } else if matcher
            .matched_path_or_any_parents(&relative, false)
            .is_ignore()
        {
            if tracked.contains(&relative) {
                return Err(HeadError::OverlayOverwritesTracked(relative));
            }
            let kind = if metadata.is_file() {
                OverlayKind::Regular {
                    identity: None,
                    requires_full_copy: false,
                }
            } else if metadata.file_type().is_symlink() {
                match plan_overlay_symlink(canonical_source_root, &source) {
                    Ok(target) => OverlayKind::Symlink { target },
                    Err(HeadError::UnsafeOverlayPath(_)) => {
                        unsafe_symlinks.insert(relative);
                        continue;
                    }
                    Err(error) => return Err(error),
                }
            } else {
                return Err(HeadError::UnsafeOverlayPath(source));
            };
            files.push(OverlayFile {
                source,
                relative,
                size: metadata.len(),
                kind,
            });
        }
    }
    Ok(())
}

fn assign_regular_identities(
    repository_root: &Path,
    files: &mut [OverlayFile],
) -> Result<(), HeadError> {
    let paths = files
        .iter()
        .filter(|file| matches!(file.kind, OverlayKind::Regular { .. }))
        .map(|file| file.source.clone())
        .collect::<Vec<_>>();
    let mut identities = hash::hash_paths(repository_root, &paths)?.into_iter();
    for file in files {
        if let OverlayKind::Regular { identity, .. } = &mut file.kind {
            *identity = Some(
                identities
                    .next()
                    .ok_or(HeadError::InvalidGitOutput("overlay hash batch"))?,
            );
        }
    }
    if identities.next().is_some() {
        return Err(HeadError::InvalidGitOutput("overlay hash batch"));
    }
    Ok(())
}

fn classify_copy_fallbacks_with(
    files: &mut [OverlayFile],
    mut can_copy_on_write: impl FnMut(&Path) -> Result<bool, HeadError>,
) -> Result<(), HeadError> {
    for file in files {
        if let OverlayKind::Regular {
            requires_full_copy, ..
        } = &mut file.kind
        {
            *requires_full_copy = !can_copy_on_write(&file.source)?;
        }
    }
    Ok(())
}

fn validate_relative_path(path: &Path) -> Result<(), HeadError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        Err(HeadError::UnsafeOverlayPath(path.to_path_buf()))
    } else {
        Ok(())
    }
}

fn plan_overlay_symlink(
    canonical_repository_root: &Path,
    source: &Path,
) -> Result<PathBuf, HeadError> {
    #[cfg(not(unix))]
    {
        let _ = canonical_repository_root;
        Err(HeadError::UnsafeOverlayPath(source.to_path_buf()))
    }

    #[cfg(unix)]
    {
        let target = fs::read_link(source).map_err(|source_error| HeadError::FileSystem {
            action: "read overlay symlink",
            path: source.to_path_buf(),
            source: source_error,
        })?;
        validate_symlink_resolution(canonical_repository_root, source, &target)?;
        Ok(target)
    }
}

fn validate_symlink_resolution(
    canonical_repository_root: &Path,
    source: &Path,
    target: &Path,
) -> Result<(), HeadError> {
    if target.is_absolute() {
        return Err(HeadError::UnsafeOverlayPath(source.to_path_buf()));
    }

    let canonical_target = crate::path::canonicalize(source)
        .map_err(|_| HeadError::UnsafeOverlayPath(source.to_path_buf()))?;
    let target_metadata =
        fs::metadata(source).map_err(|_| HeadError::UnsafeOverlayPath(source.to_path_buf()))?;
    if !canonical_target.starts_with(canonical_repository_root)
        || (!target_metadata.is_file() && !target_metadata.is_dir())
    {
        return Err(HeadError::UnsafeOverlayPath(source.to_path_buf()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        CopyOnWriteProbe, OverlayFile, OverlayKind, classify_copy_fallbacks_with,
        materialization::{overlay_parent_paths, verify_regular_overlay_identities},
    };

    fn overlay_file(name: &str, size: u64) -> OverlayFile {
        OverlayFile {
            source: PathBuf::from(name),
            relative: PathBuf::from(name),
            size,
            kind: OverlayKind::Regular {
                identity: Some("identity".to_owned()),
                requires_full_copy: false,
            },
        }
    }

    fn overlay_symlink(name: &str, target: &str) -> OverlayFile {
        OverlayFile {
            source: PathBuf::from(name),
            relative: PathBuf::from(name),
            size: target.len() as u64,
            kind: OverlayKind::Symlink {
                target: PathBuf::from(target),
            },
        }
    }

    #[test]
    fn copy_on_write_overlays_do_not_require_confirmation() {
        let mut files = vec![overlay_file(".env", 7), overlay_file("cache.bin", 11)];

        classify_copy_fallbacks_with(&mut files, |_| Ok(true))
            .expect("copy-on-write assessment should succeed");

        assert!(files.iter().all(|file| !file.requires_full_copy()));
    }

    #[test]
    fn only_overlays_that_need_full_copy_are_marked_for_confirmation() {
        let mut files = vec![overlay_file(".env", 7), overlay_file("cache.bin", 11)];
        let mut assessments = [true, false].into_iter();

        classify_copy_fallbacks_with(&mut files, |_| {
            Ok(assessments.next().expect("one assessment per overlay"))
        })
        .expect("fallback assessment should succeed");

        assert!(!files[0].requires_full_copy());
        assert!(files[1].requires_full_copy());
    }

    #[test]
    fn symlink_overlays_do_not_require_a_copy_fallback_probe() {
        let mut files = vec![overlay_symlink(
            "node_modules/.bin/acorn",
            "../acorn/bin/acorn",
        )];
        let mut probes = 0;

        classify_copy_fallbacks_with(&mut files, |_| {
            probes += 1;
            Ok(false)
        })
        .expect("symlink assessment should succeed");

        assert_eq!(probes, 0);
        assert!(!files[0].requires_full_copy());
    }

    #[test]
    fn a_successful_probe_does_not_authorize_a_later_file_without_testing_it() {
        let mut files = vec![overlay_file("deps/one", 7), overlay_file("deps/two", 11)];
        let mut probes = 0;
        let mut assessments = [true, false].into_iter();

        classify_copy_fallbacks_with(&mut files, |_| {
            probes += 1;
            Ok(assessments.next().expect("one assessment per regular file"))
        })
        .expect("per-file assessment should succeed");

        assert_eq!(probes, 2);
        assert!(!files[0].requires_full_copy());
        assert!(files[1].requires_full_copy());
    }

    #[test]
    fn a_failed_probe_is_not_reused_for_other_files() {
        let mut files = vec![
            overlay_file("deps/one", 7),
            overlay_file("deps/two", 11),
            overlay_file("deps/three", 13),
        ];
        let mut probes = 0;

        classify_copy_fallbacks_with(&mut files, |_| {
            probes += 1;
            Ok(false)
        })
        .expect("fallback assessment should succeed");

        assert_eq!(probes, 3);
        assert!(files.iter().all(OverlayFile::requires_full_copy));
    }

    #[test]
    fn overlay_parent_directories_are_deduplicated_before_materialization() {
        let files = vec![
            overlay_file("deps/one", 7),
            overlay_file("deps/two", 11),
            overlay_file("deps/nested/three", 13),
        ];

        let parents = overlay_parent_paths(&files).expect("overlay parents should be safe");

        assert_eq!(
            parents,
            [PathBuf::from("deps"), PathBuf::from("deps/nested"),]
        );
    }

    #[test]
    fn a_materialized_overlay_matching_its_planned_identity_survives_later_source_removal() {
        let temporary = tempfile::tempdir().expect("temporary directory should be created");
        let repository = temporary.path().join("repository");
        let head = temporary.path().join("head");
        std::fs::create_dir_all(repository.join("deps")).expect("source parent should be created");
        std::fs::create_dir_all(head.join("deps")).expect("Head parent should be created");
        let source = repository.join("deps/package");
        let destination = head.join("deps/package");
        std::fs::write(&source, b"planned bytes").expect("source should be written");
        std::fs::write(&destination, b"planned bytes").expect("destination should be written");
        let identity = super::hash::hash_paths(&repository, std::slice::from_ref(&source))
            .expect("planned identity should be computed")
            .remove(0);
        std::fs::remove_file(&source).expect("source should be removed after materialization");
        let file = OverlayFile {
            source,
            relative: PathBuf::from("deps/package"),
            size: 13,
            kind: OverlayKind::Regular {
                identity: Some(identity),
                requires_full_copy: false,
            },
        };

        verify_regular_overlay_identities(&repository, &head, &[&file])
            .expect("the verified Head payload should not depend on the later source state");
    }

    #[test]
    fn a_materialized_overlay_that_differs_from_its_planned_identity_is_rejected() {
        let temporary = tempfile::tempdir().expect("temporary directory should be created");
        let repository = temporary.path().join("repository");
        let head = temporary.path().join("head");
        std::fs::create_dir_all(repository.join("deps")).expect("source parent should be created");
        std::fs::create_dir_all(head.join("deps")).expect("Head parent should be created");
        let source = repository.join("deps/package");
        let destination = head.join("deps/package");
        std::fs::write(&source, b"planned bytes").expect("source should be written");
        let identity = super::hash::hash_paths(&repository, std::slice::from_ref(&source))
            .expect("planned identity should be computed")
            .remove(0);
        std::fs::write(&destination, b"different bytes")
            .expect("different destination should be written");
        let file = OverlayFile {
            source: source.clone(),
            relative: PathBuf::from("deps/package"),
            size: 13,
            kind: OverlayKind::Regular {
                identity: Some(identity),
                requires_full_copy: false,
            },
        };

        let error = verify_regular_overlay_identities(&repository, &head, &[&file])
            .expect_err("a materialized payload mismatch should be rejected");

        assert!(matches!(
            error,
            super::HeadError::OverlayChanged(path) if path == source
        ));
    }

    #[test]
    fn copy_on_write_probe_owns_and_removes_an_isolated_directory() {
        let temporary = tempfile::tempdir().expect("temporary directory should be created");
        let source = temporary.path().join("source");
        std::fs::write(&source, b"probe").expect("probe source should be written");
        let mut probe =
            CopyOnWriteProbe::create(temporary.path()).expect("probe directory should be created");
        let directory = probe.directory.clone();
        let destination = probe.destination.clone();

        let _ = probe
            .assess(&source)
            .expect("copy-on-write assessment should complete");

        assert!(directory.is_dir());
        assert!(!destination.exists());
        probe.finish().expect("probe directory should be removed");
        assert!(!directory.exists());
    }
}

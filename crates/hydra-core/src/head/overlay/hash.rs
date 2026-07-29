use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

use super::HeadError;

const MAX_HASH_BATCH_PATHS: usize = 512;
const MAX_HASH_BATCH_BYTES: usize = 64 * 1_024;
const MAX_HASH_WORKERS: usize = 8;

pub(super) fn hash_paths(
    repository_root: &Path,
    paths: &[PathBuf],
) -> Result<Vec<String>, HeadError> {
    let workers = thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(MAX_HASH_WORKERS);
    hash_paths_parallel_with(paths, workers, |batch| {
        run_hash_batch(repository_root, batch)
    })
}

fn hash_paths_parallel_with(
    paths: &[PathBuf],
    workers: usize,
    hash_batch: impl Fn(&[PathBuf]) -> Result<Vec<String>, HeadError> + Sync,
) -> Result<Vec<String>, HeadError> {
    let mut batches = Vec::new();
    let mut start = 0;
    while start < paths.len() {
        let end = batch_end(paths, start);
        batches.push((start, end));
        start = end;
    }
    if batches.len() <= 1 || workers <= 1 {
        return hash_paths_with(paths, |batch| hash_batch(batch));
    }

    let worker_count = workers.min(batches.len());
    let next_batch = AtomicUsize::new(0);
    let completed = thread::scope(|scope| {
        let mut handles = Vec::with_capacity(worker_count);
        for worker in 0..worker_count {
            handles.push(
                thread::Builder::new()
                    .name(format!("hydra-overlay-hash-{worker}"))
                    .spawn_scoped(scope, || {
                        let mut completed = Vec::new();
                        loop {
                            let index = next_batch.fetch_add(1, Ordering::Relaxed);
                            let Some(&(start, end)) = batches.get(index) else {
                                break;
                            };
                            completed.push((
                                index,
                                hash_batch_with_retry(&paths[start..end], &hash_batch),
                            ));
                        }
                        completed
                    })
                    .map_err(|source| HeadError::FileSystem {
                        action: "start overlay hash worker",
                        path: paths[0].clone(),
                        source,
                    })?,
            );
        }

        let mut completed = Vec::with_capacity(batches.len());
        for handle in handles {
            completed.extend(
                handle
                    .join()
                    .map_err(|_| HeadError::InvalidGitOutput("overlay hash worker"))?,
            );
        }
        Ok::<_, HeadError>(completed)
    })?;

    let mut ordered = std::iter::repeat_with(|| None)
        .take(batches.len())
        .collect::<Vec<_>>();
    for (index, result) in completed {
        ordered[index] = Some(result);
    }
    let mut hashes = Vec::with_capacity(paths.len());
    for result in ordered {
        hashes.extend(result.ok_or(HeadError::InvalidGitOutput("overlay hash worker"))??);
    }
    Ok(hashes)
}

fn hash_batch_with_retry(
    paths: &[PathBuf],
    hash_batch: &(impl Fn(&[PathBuf]) -> Result<Vec<String>, HeadError> + Sync),
) -> Result<Vec<String>, HeadError> {
    match hash_batch(paths) {
        Ok(hashes) if hashes.len() == paths.len() => Ok(hashes),
        Ok(_) => Err(HeadError::InvalidGitOutput("overlay hash batch")),
        Err(HeadError::GitUnavailable(error))
            if error.kind() == std::io::ErrorKind::ArgumentListTooLong && paths.len() > 1 =>
        {
            let middle = paths.len() / 2;
            let mut hashes = hash_batch_with_retry(&paths[..middle], hash_batch)?;
            hashes.extend(hash_batch_with_retry(&paths[middle..], hash_batch)?);
            Ok(hashes)
        }
        Err(error) => Err(error),
    }
}

fn hash_paths_with(
    paths: &[PathBuf],
    mut hash_batch: impl FnMut(&[PathBuf]) -> Result<Vec<String>, HeadError>,
) -> Result<Vec<String>, HeadError> {
    let mut hashes = Vec::with_capacity(paths.len());
    let mut start = 0;
    while start < paths.len() {
        let end = batch_end(paths, start);
        let batch = &paths[start..end];
        append_hash_batch(batch, &mut hash_batch, &mut hashes)?;
        start = end;
    }
    Ok(hashes)
}

fn append_hash_batch(
    paths: &[PathBuf],
    hash_batch: &mut impl FnMut(&[PathBuf]) -> Result<Vec<String>, HeadError>,
    hashes: &mut Vec<String>,
) -> Result<(), HeadError> {
    match hash_batch(paths) {
        Ok(batch_hashes) => {
            if batch_hashes.len() != paths.len() {
                return Err(HeadError::InvalidGitOutput("overlay hash batch"));
            }
            hashes.extend(batch_hashes);
            Ok(())
        }
        Err(HeadError::GitUnavailable(error))
            if error.kind() == std::io::ErrorKind::ArgumentListTooLong && paths.len() > 1 =>
        {
            let middle = paths.len() / 2;
            append_hash_batch(&paths[..middle], hash_batch, hashes)?;
            append_hash_batch(&paths[middle..], hash_batch, hashes)
        }
        Err(error) => Err(error),
    }
}

fn batch_end(paths: &[PathBuf], start: usize) -> usize {
    let mut bytes = 0_usize;
    let mut end = start;
    while end < paths.len() && end - start < MAX_HASH_BATCH_PATHS {
        let path_bytes = encoded_path_len(&paths[end]).saturating_add(1);
        if end > start && bytes.saturating_add(path_bytes) > MAX_HASH_BATCH_BYTES {
            break;
        }
        bytes = bytes.saturating_add(path_bytes);
        end += 1;
    }
    end
}

#[cfg(unix)]
fn encoded_path_len(path: &Path) -> usize {
    path.as_os_str().as_bytes().len()
}

#[cfg(windows)]
fn encoded_path_len(path: &Path) -> usize {
    path.as_os_str().encode_wide().count().saturating_mul(2)
}

#[cfg(not(any(unix, windows)))]
fn encoded_path_len(path: &Path) -> usize {
    path.as_os_str().to_string_lossy().len()
}

fn run_hash_batch(repository_root: &Path, paths: &[PathBuf]) -> Result<Vec<String>, HeadError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(["hash-object", "--no-filters", "--"])
        .args(paths)
        .output()
        .map_err(HeadError::GitUnavailable)?;
    if !output.status.success() {
        return Err(HeadError::GitCommandFailed {
            operation: "hashing overlay files",
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| HeadError::InvalidGitOutput("overlay hash batch"))?;
    let hashes = stdout.lines().map(str::to_owned).collect::<Vec<String>>();
    if hashes.iter().any(|hash| {
        !matches!(hash.len(), 40 | 64) || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        return Err(HeadError::InvalidGitOutput("overlay hash batch"));
    }
    Ok(hashes)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        thread,
        time::Duration,
    };

    use super::{hash_paths, hash_paths_parallel_with, hash_paths_with};

    #[test]
    fn hashing_many_paths_uses_bounded_batches_and_preserves_order() {
        let paths = (0..1_025)
            .map(|index| PathBuf::from(format!("deps/file-{index}")))
            .collect::<Vec<_>>();
        let mut batch_sizes = Vec::new();

        let hashes = hash_paths_with(&paths, |batch| {
            batch_sizes.push(batch.len());
            Ok(batch
                .iter()
                .map(|path| format!("hash:{}", path.display()))
                .collect())
        })
        .expect("batched hashing should succeed");

        assert_eq!(batch_sizes, [512, 512, 1]);
        assert_eq!(hashes.len(), paths.len());
        assert_eq!(hashes[0], "hash:deps/file-0");
        assert_eq!(hashes[1_024], "hash:deps/file-1024");
    }

    #[test]
    fn independent_hash_batches_run_concurrently_without_reordering_results() {
        let paths = (0..1_025)
            .map(|index| PathBuf::from(format!("deps/file-{index}")))
            .collect::<Vec<_>>();
        let active = Arc::new(AtomicUsize::new(0));
        let maximum_active = Arc::new(AtomicUsize::new(0));

        let hashes = hash_paths_parallel_with(&paths, 3, {
            let active = Arc::clone(&active);
            let maximum_active = Arc::clone(&maximum_active);
            move |batch| {
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                maximum_active.fetch_max(current, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(20));
                active.fetch_sub(1, Ordering::SeqCst);
                Ok(batch
                    .iter()
                    .map(|path| format!("hash:{}", path.display()))
                    .collect())
            }
        })
        .expect("parallel hashing should succeed");

        assert_eq!(hashes.len(), paths.len());
        assert_eq!(hashes[0], "hash:deps/file-0");
        assert_eq!(hashes[512], "hash:deps/file-512");
        assert_eq!(hashes[1_024], "hash:deps/file-1024");
        assert!(maximum_active.load(Ordering::SeqCst) > 1);
    }

    #[test]
    fn parallel_hashing_retries_argument_limits_without_reordering_results() {
        let paths = (0..1_025)
            .map(|index| PathBuf::from(format!("deps/file-{index}")))
            .collect::<Vec<_>>();

        let hashes = hash_paths_parallel_with(&paths, 3, |batch| {
            if batch.len() > 128 {
                Err(super::HeadError::GitUnavailable(std::io::Error::from(
                    std::io::ErrorKind::ArgumentListTooLong,
                )))
            } else {
                Ok(batch
                    .iter()
                    .map(|path| format!("hash:{}", path.display()))
                    .collect())
            }
        })
        .expect("parallel argument-limit retries should succeed");

        assert_eq!(hashes.len(), paths.len());
        assert_eq!(hashes[0], "hash:deps/file-0");
        assert_eq!(hashes[512], "hash:deps/file-512");
        assert_eq!(hashes[1_024], "hash:deps/file-1024");
    }

    #[test]
    fn parallel_hashing_rejects_a_batch_with_the_wrong_result_count() {
        let paths = (0..1_025)
            .map(|index| PathBuf::from(format!("deps/file-{index}")))
            .collect::<Vec<_>>();

        let error = hash_paths_parallel_with(&paths, 3, |batch| {
            let result_count = if batch[0] == std::path::Path::new("deps/file-512") {
                batch.len() - 1
            } else {
                batch.len()
            };
            Ok(vec!["hash".to_owned(); result_count])
        })
        .expect_err("a malformed parallel batch should be rejected");

        assert!(matches!(
            error,
            super::HeadError::InvalidGitOutput("overlay hash batch")
        ));
    }

    #[test]
    fn hashing_batches_split_before_the_argument_budget() {
        let paths = vec![
            PathBuf::from("a".repeat(40_000)),
            PathBuf::from("b".repeat(40_000)),
        ];
        let mut batch_sizes = Vec::new();

        hash_paths_with(&paths, |batch| {
            batch_sizes.push(batch.len());
            Ok(vec!["hash".to_owned(); batch.len()])
        })
        .expect("budgeted hashing should succeed");

        assert_eq!(batch_sizes, [1, 1]);
    }

    #[test]
    fn hashing_rejects_a_batch_with_the_wrong_number_of_results() {
        let paths = vec![PathBuf::from("one"), PathBuf::from("two")];

        let error = hash_paths_with(&paths, |_| Ok(vec!["only-one".to_owned()]))
            .expect_err("missing hash output should be rejected");

        assert!(matches!(
            error,
            super::HeadError::InvalidGitOutput("overlay hash batch")
        ));
    }

    #[test]
    fn argument_list_limits_retry_the_same_paths_in_smaller_ordered_batches() {
        let paths = (0..5)
            .map(|index| PathBuf::from(format!("file-{index}")))
            .collect::<Vec<_>>();
        let mut attempted_batch_sizes = Vec::new();

        let hashes = hash_paths_with(&paths, |batch| {
            attempted_batch_sizes.push(batch.len());
            if batch.len() > 2 {
                Err(super::HeadError::GitUnavailable(std::io::Error::from(
                    std::io::ErrorKind::ArgumentListTooLong,
                )))
            } else {
                Ok(batch
                    .iter()
                    .map(|path| format!("hash:{}", path.display()))
                    .collect())
            }
        })
        .expect("argument-limit retries should succeed");

        assert_eq!(attempted_batch_sizes, [5, 2, 3, 1, 2]);
        assert_eq!(
            hashes,
            [
                "hash:file-0",
                "hash:file-1",
                "hash:file-2",
                "hash:file-3",
                "hash:file-4",
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn git_hash_batches_preserve_paths_with_newlines_and_leading_dashes() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let paths = [
            PathBuf::from("line\nbreak"),
            PathBuf::from("-leading-option"),
        ];
        fs::write(directory.path().join(&paths[0]), b"first\n")
            .expect("first hash fixture should be written");
        fs::write(directory.path().join(&paths[1]), b"second\n")
            .expect("second hash fixture should be written");

        let hashes =
            hash_paths(directory.path(), &paths).expect("unusual paths should be hashable");

        assert_eq!(hashes.len(), paths.len());
        assert!(hashes.iter().all(|hash| hash.len() == 40));
        assert_ne!(hashes[0], hashes[1]);
    }

    #[cfg(unix)]
    #[test]
    fn batching_preserves_non_utf8_paths_without_lossy_conversion() {
        use std::{
            ffi::OsString,
            os::unix::ffi::{OsStrExt, OsStringExt},
        };

        let path = PathBuf::from(OsString::from_vec(b"non-\xff-utf8".to_vec()));

        let hashes = hash_paths_with(&[path], |batch| {
            assert_eq!(batch[0].as_os_str().as_bytes(), b"non-\xff-utf8");
            Ok(vec!["preserved".to_owned()])
        })
        .expect("non-UTF-8 path should be preserved");

        assert_eq!(hashes, ["preserved"]);
    }
}

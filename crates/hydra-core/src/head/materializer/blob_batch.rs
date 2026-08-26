use std::{
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::Path,
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
    thread::{self, JoinHandle},
};

use super::{HeadError, Repository};

const MAX_HEADER_BYTES: usize = 256;
const STREAM_BUFFER_BYTES: usize = 64 * 1024;
const MAX_CAPTURED_STDERR_BYTES: usize = 64 * 1024;

pub(super) struct GitBlobBatch {
    child: Option<Child>,
    input: Option<BufWriter<ChildStdin>>,
    output: BufReader<ChildStdout>,
    stderr_drain: Option<JoinHandle<std::io::Result<Vec<u8>>>>,
    stream_buffer: Box<[u8]>,
}

impl GitBlobBatch {
    pub(super) fn start(repository: &Repository) -> Result<Self, HeadError> {
        let mut child = Command::new("git")
            .arg("-C")
            .arg(&repository.root)
            .args(["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(HeadError::GitUnavailable)?;
        let Some(input) = child.stdin.take() else {
            terminate_child(&mut child);
            return Err(HeadError::InvalidGitOutput("tracked blob batch input"));
        };
        let Some(output) = child.stdout.take() else {
            terminate_child(&mut child);
            return Err(HeadError::InvalidGitOutput("tracked blob batch output"));
        };
        let Some(stderr) = child.stderr.take() else {
            terminate_child(&mut child);
            return Err(HeadError::InvalidGitOutput(
                "tracked blob batch error output",
            ));
        };
        let stderr_drain = match thread::Builder::new()
            .name("hydra-git-blob-stderr".to_owned())
            .spawn(move || drain_stderr(stderr))
        {
            Ok(stderr_drain) => stderr_drain,
            Err(source) => {
                terminate_child(&mut child);
                return Err(git_io_failure(
                    "starting tracked Git blob error reader",
                    &source,
                ));
            }
        };
        Ok(Self {
            child: Some(child),
            input: Some(BufWriter::new(input)),
            output: BufReader::new(output),
            stderr_drain: Some(stderr_drain),
            stream_buffer: vec![0_u8; STREAM_BUFFER_BYTES].into_boxed_slice(),
        })
    }

    pub(super) fn write_blob(
        &mut self,
        object: &str,
        destination: &mut impl Write,
        destination_path: &Path,
    ) -> Result<(), HeadError> {
        let size = self.begin_blob(object)?;
        self.read_payload(size, |bytes| {
            destination
                .write_all(bytes)
                .map_err(|source| HeadError::FileSystem {
                    action: "write temporary Git blob",
                    path: destination_path.to_path_buf(),
                    source,
                })
        })
    }

    #[cfg(any(unix, test))]
    pub(super) fn read_blob(&mut self, object: &str) -> Result<Vec<u8>, HeadError> {
        let size = self.begin_blob(object)?;
        let capacity = usize::try_from(size)
            .map_err(|_| HeadError::InvalidGitOutput("tracked blob batch size"))?;
        let mut contents = Vec::new();
        contents
            .try_reserve_exact(capacity)
            .map_err(|_| HeadError::InvalidGitOutput("tracked blob batch size"))?;
        self.read_payload(size, |bytes| {
            contents.extend_from_slice(bytes);
            Ok(())
        })?;
        Ok(contents)
    }

    pub(super) fn finish(mut self) -> Result<(), HeadError> {
        drop(self.input.take());
        let mut child = self
            .child
            .take()
            .ok_or(HeadError::InvalidGitOutput("tracked blob batch process"))?;
        let status = match child.wait() {
            Ok(status) => status,
            Err(source) => {
                terminate_child(&mut child);
                return Err(git_io_failure("waiting for tracked Git blobs", &source));
            }
        };
        let stderr = self.join_stderr_drain()?;
        if status.success() {
            Ok(())
        } else {
            Err(HeadError::GitCommandFailed {
                operation: "reading tracked Git blobs",
                status: status.code(),
                stderr: String::from_utf8_lossy(&stderr).into_owned(),
            })
        }
    }

    fn begin_blob(&mut self, object: &str) -> Result<u64, HeadError> {
        validate_object_id(object)?;
        let input = self
            .input
            .as_mut()
            .ok_or(HeadError::InvalidGitOutput("tracked blob batch input"))?;
        input
            .write_all(object.as_bytes())
            .and_then(|()| input.write_all(b"\n"))
            .and_then(|()| input.flush())
            .map_err(|source| git_io_failure("requesting a tracked Git blob", &source))?;
        let header = read_bounded_header(&mut self.output)?;
        parse_blob_header(&header, object)
    }

    fn read_payload(
        &mut self,
        size: u64,
        mut consume: impl FnMut(&[u8]) -> Result<(), HeadError>,
    ) -> Result<(), HeadError> {
        let mut remaining = size;
        while remaining > 0 {
            let length = usize::try_from(remaining.min(self.stream_buffer.len() as u64))
                .map_err(|_| HeadError::InvalidGitOutput("tracked blob batch size"))?;
            self.output
                .read_exact(&mut self.stream_buffer[..length])
                .map_err(|source| git_io_failure("reading a tracked Git blob", &source))?;
            consume(&self.stream_buffer[..length])?;
            remaining -= length as u64;
        }
        let mut terminator = [0_u8; 1];
        self.output
            .read_exact(&mut terminator)
            .map_err(|source| git_io_failure("reading tracked Git blob framing", &source))?;
        if terminator != *b"\n" {
            return Err(HeadError::InvalidGitOutput(
                "tracked blob batch payload framing",
            ));
        }
        Ok(())
    }

    fn join_stderr_drain(&mut self) -> Result<Vec<u8>, HeadError> {
        let drain = self.stderr_drain.take().ok_or(HeadError::InvalidGitOutput(
            "tracked blob batch error reader",
        ))?;
        drain
            .join()
            .map_err(|_| HeadError::InvalidGitOutput("tracked blob batch error reader"))?
            .map_err(|source| git_io_failure("reading tracked Git blob errors", &source))
    }
}

impl Drop for GitBlobBatch {
    fn drop(&mut self) {
        drop(self.input.take());
        if let Some(mut child) = self.child.take() {
            terminate_child(&mut child);
        }
        if let Some(stderr_drain) = self.stderr_drain.take() {
            let _ = stderr_drain.join();
        }
    }
}

fn validate_object_id(object: &str) -> Result<(), HeadError> {
    if matches!(object.len(), 40 | 64) && object.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(HeadError::InvalidGitOutput("tracked blob object id"))
    }
}

fn read_bounded_header(reader: &mut impl BufRead) -> Result<Vec<u8>, HeadError> {
    let mut header = Vec::with_capacity(96);
    loop {
        let available = reader
            .fill_buf()
            .map_err(|source| git_io_failure("reading a tracked Git blob header", &source))?;
        if available.is_empty() {
            return Err(HeadError::InvalidGitOutput("tracked blob batch header"));
        }
        let length = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        if header.len() + length > MAX_HEADER_BYTES {
            return Err(HeadError::InvalidGitOutput("tracked blob batch header"));
        }
        let finished = available[length - 1] == b'\n';
        header.extend_from_slice(&available[..length]);
        reader.consume(length);
        if finished {
            return Ok(header);
        }
    }
}

fn parse_blob_header(header: &[u8], requested_object: &str) -> Result<u64, HeadError> {
    let header = header
        .strip_suffix(b"\n")
        .ok_or(HeadError::InvalidGitOutput("tracked blob batch header"))?;
    let header = std::str::from_utf8(header)
        .map_err(|_| HeadError::InvalidGitOutput("tracked blob batch header"))?;
    let mut fields = header.split(' ');
    let object = fields
        .next()
        .ok_or(HeadError::InvalidGitOutput("tracked blob batch header"))?;
    let kind = fields
        .next()
        .ok_or(HeadError::InvalidGitOutput("tracked blob batch header"))?;
    let size = fields
        .next()
        .ok_or(HeadError::InvalidGitOutput("tracked blob batch header"))?;
    if fields.next().is_some()
        || !object.eq_ignore_ascii_case(requested_object)
        || kind != "blob"
        || size.is_empty()
    {
        return Err(HeadError::InvalidGitOutput("tracked blob batch header"));
    }
    size.parse::<u64>()
        .map_err(|_| HeadError::InvalidGitOutput("tracked blob batch size"))
}

fn git_io_failure(operation: &'static str, source: &std::io::Error) -> HeadError {
    HeadError::GitCommandFailed {
        operation,
        status: None,
        stderr: source.to_string(),
    }
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn drain_stderr(mut stderr: ChildStderr) -> std::io::Result<Vec<u8>> {
    let mut captured = Vec::new();
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let length = stderr.read(&mut buffer)?;
        if length == 0 {
            return Ok(captured);
        }
        let remaining = MAX_CAPTURED_STDERR_BYTES.saturating_sub(captured.len());
        captured.extend_from_slice(&buffer[..length.min(remaining)]);
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, process::Command};

    use tempfile::tempdir;

    use super::{GitBlobBatch, parse_blob_header};
    use crate::head::git::Repository;

    #[test]
    fn batch_header_reports_the_exact_requested_blob_size() {
        let object = "0123456789012345678901234567890123456789";
        let header = format!("{object} blob 17\n");

        let size = parse_blob_header(header.as_bytes(), object)
            .expect("valid batch header should be accepted");

        assert_eq!(size, 17);
    }

    #[test]
    fn batch_header_rejects_a_different_object_or_non_blob_type() {
        let object = "0123456789012345678901234567890123456789";

        assert!(parse_blob_header(b"abcdef tree 17\n", object).is_err());
        assert!(parse_blob_header(b"abcdef missing\n", object).is_err());
        assert!(parse_blob_header(format!("{object} blob nope\n").as_bytes(), object).is_err());
        assert!(parse_blob_header(format!("{object} blob 17 extra\n").as_bytes(), object).is_err());
        assert!(parse_blob_header(format!("{object} blob 17").as_bytes(), object).is_err());
    }

    #[test]
    fn batch_header_accepts_a_sha256_object_id() {
        let object = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let header = format!("{object} blob 4294967296\n");

        let size = parse_blob_header(header.as_bytes(), object)
            .expect("SHA-256 batch header should be accepted");

        assert_eq!(size, 4_294_967_296);
    }

    #[test]
    fn one_batch_process_reads_multiple_blobs_in_request_order() {
        let temporary = tempdir().expect("temporary directory should be created");
        run_git(temporary.path(), &["init", "--quiet"]);
        let first_contents = b"first blob\n";
        let second_contents = b"second\0blob\n";
        fs::write(temporary.path().join("first"), first_contents)
            .expect("first fixture should be written");
        fs::write(temporary.path().join("second"), second_contents)
            .expect("second fixture should be written");
        let first_object = git_stdout(temporary.path(), &["hash-object", "-w", "first"]);
        let second_object = git_stdout(temporary.path(), &["hash-object", "-w", "second"]);
        let repository = Repository {
            root: temporary.path().to_path_buf(),
            git_common_directory: temporary.path().join(".git"),
        };

        let mut batch = GitBlobBatch::start(&repository).expect("batch reader should start");
        let first = batch
            .read_blob(&first_object)
            .expect("first blob should be read");
        let mut second = Vec::new();
        batch
            .write_blob(
                &second_object,
                &mut second,
                Path::new("temporary destination"),
            )
            .expect("second blob should be streamed");
        batch.finish().expect("batch reader should finish cleanly");

        assert_eq!(first, first_contents);
        assert_eq!(second, second_contents);
    }

    #[test]
    fn invalid_object_id_is_rejected_without_poisoning_the_batch() {
        let temporary = tempdir().expect("temporary directory should be created");
        run_git(temporary.path(), &["init", "--quiet"]);
        fs::write(temporary.path().join("blob"), b"valid").expect("blob fixture should be written");
        let object = git_stdout(temporary.path(), &["hash-object", "-w", "blob"]);
        let repository = Repository {
            root: temporary.path().to_path_buf(),
            git_common_directory: temporary.path().join(".git"),
        };

        let mut batch = GitBlobBatch::start(&repository).expect("batch reader should start");
        assert!(batch.read_blob("../HEAD\n").is_err());
        assert_eq!(
            batch
                .read_blob(&object)
                .expect("valid request should still work"),
            b"valid"
        );
        batch.finish().expect("batch reader should finish cleanly");
    }

    #[test]
    fn a_destination_write_error_terminates_an_unfinished_large_blob_without_hanging() {
        struct RejectWrites;

        impl std::io::Write for RejectWrites {
            fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::StorageFull,
                    "simulated destination failure",
                ))
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let temporary = tempdir().expect("temporary directory should be created");
        run_git(temporary.path(), &["init", "--quiet"]);
        fs::write(temporary.path().join("large"), vec![b'x'; 256 * 1024])
            .expect("large blob fixture should be written");
        let object = git_stdout(temporary.path(), &["hash-object", "-w", "large"]);
        let repository = Repository {
            root: temporary.path().to_path_buf(),
            git_common_directory: temporary.path().join(".git"),
        };
        let mut batch = GitBlobBatch::start(&repository).expect("batch reader should start");

        let error = batch
            .write_blob(&object, &mut RejectWrites, Path::new("full destination"))
            .expect_err("destination failure should abort the stream");

        assert!(matches!(error, super::HeadError::FileSystem { .. }));
        drop(batch);
    }

    fn run_git(path: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(arguments)
            .output()
            .expect("Git should run");
        assert!(
            output.status.success(),
            "Git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_stdout(path: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(arguments)
            .output()
            .expect("Git should run");
        assert!(
            output.status.success(),
            "Git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("Git output should be UTF-8")
            .trim_end()
            .to_owned()
    }
}

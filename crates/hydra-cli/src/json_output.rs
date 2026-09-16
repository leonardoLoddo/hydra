use std::io::{self, Write as _};

use serde::Serialize;

pub(super) fn write(value: &impl Serialize) -> Result<(), String> {
    let mut bytes = serde_json::to_vec(value)
        .map_err(|error| format!("could not serialize JSON output: {error}"))?;
    bytes.push(b'\n');
    io::stdout()
        .lock()
        .write_all(&bytes)
        .map_err(|error| format!("could not write JSON output: {error}"))
}

pub(super) fn path(path: &std::path::Path) -> Result<&str, String> {
    path.to_str().ok_or_else(|| {
        "JSON output cannot represent a filesystem path that is not valid Unicode".to_owned()
    })
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    #[test]
    fn rejects_non_unicode_paths_instead_of_serializing_a_lossy_value() {
        use super::path;
        use std::{ffi::OsString, os::unix::ffi::OsStringExt as _, path::PathBuf};

        let value = PathBuf::from(OsString::from_vec(vec![b'/', 0xff]));

        assert_eq!(
            path(&value),
            Err(
                "JSON output cannot represent a filesystem path that is not valid Unicode"
                    .to_owned()
            )
        );
    }
}

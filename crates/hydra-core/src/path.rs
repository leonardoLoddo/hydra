use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(crate) fn canonicalize(path: &Path) -> io::Result<PathBuf> {
    fs::canonicalize(path).map(simplify_verbatim_prefix)
}

#[cfg(not(windows))]
fn simplify_verbatim_prefix(path: PathBuf) -> PathBuf {
    path
}

#[cfg(windows)]
fn simplify_verbatim_prefix(path: PathBuf) -> PathBuf {
    use std::path::{Component, Prefix};

    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return path;
    };
    let mut simplified = match prefix.kind() {
        Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:", char::from(drive))),
        Prefix::VerbatimUNC(server, share) => {
            let mut root = PathBuf::from(r"\\");
            root.push(server);
            root.push(share);
            root
        }
        _ => return path,
    };
    simplified.extend(components);
    simplified
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn windows_verbatim_disk_paths_become_git_compatible() {
        let path = super::simplify_verbatim_prefix(std::path::PathBuf::from(
            r"\\?\C:\projects\hydra.heads",
        ));

        assert_eq!(path, std::path::Path::new(r"C:\projects\hydra.heads"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_verbatim_unc_paths_become_git_compatible() {
        let path = super::simplify_verbatim_prefix(std::path::PathBuf::from(
            r"\\?\UNC\server\share\hydra.heads",
        ));

        assert_eq!(path, std::path::Path::new(r"\\server\share\hydra.heads"));
    }
}

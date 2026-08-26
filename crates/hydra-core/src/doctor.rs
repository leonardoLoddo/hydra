use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

#[cfg(target_os = "linux")]
use std::{ffi::OsString, os::unix::ffi::OsStringExt};

use uuid::Uuid;

use crate::{
    HeadError, InitError, StorageBackend, head,
    init::storage::{probe_full_copy, probe_storage},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeStoragePrimitive {
    ApfsClone,
    LinuxReflink,
    WindowsReFsBlockClone,
    NativeClone,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageEnvironment {
    Native,
    WindowsSubsystemForLinux,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativePlatform {
    MacOs,
    Linux,
    Windows,
    Other,
}

#[derive(Debug)]
pub struct StorageDiagnostics {
    pub storage_backend: StorageBackend,
    pub native_primitive: NativeStoragePrimitive,
    pub environment: StorageEnvironment,
    pub filesystem: Option<String>,
    pub full_copy_fallback_verified: bool,
    pub mutable_hard_links_enabled: bool,
    pub isolation_supported: bool,
}

#[derive(Debug)]
pub enum DoctorError {
    Project(HeadError),
    Probe(InitError),
    FileSystem {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    ProbeCleanupFailed {
        probe: Box<InitError>,
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Probes native cloning and the full-copy fallback on the managed Heads volume.
///
/// # Errors
///
/// Returns [`DoctorError`] when the Hydra installation is invalid, a temporary
/// probe directory cannot be managed, either storage probe fails, or cleanup
/// cannot remove every test-owned artifact.
pub fn diagnose_storage(source_path: &Path) -> Result<StorageDiagnostics, DoctorError> {
    let heads_directory =
        head::validated_heads_directory(source_path).map_err(DoctorError::Project)?;
    let probe_directory =
        heads_directory.join(format!(".hydra-storage-doctor-{}", Uuid::new_v4().simple()));
    fs::create_dir(&probe_directory).map_err(|source| DoctorError::FileSystem {
        action: "create temporary storage diagnostic directory",
        path: probe_directory.clone(),
        source,
    })?;

    let probe = run_probes(&probe_directory);
    let cleanup = fs::remove_dir(&probe_directory);
    match (probe, cleanup) {
        (Ok(diagnostics), Ok(())) => Ok(diagnostics),
        (Err(probe), Ok(())) => Err(DoctorError::Probe(probe)),
        (Ok(_), Err(source)) => Err(DoctorError::FileSystem {
            action: "remove temporary storage diagnostic directory",
            path: probe_directory,
            source,
        }),
        (Err(probe), Err(source)) => Err(DoctorError::ProbeCleanupFailed {
            probe: Box::new(probe),
            path: probe_directory,
            source,
        }),
    }
}

fn run_probes(destination: &Path) -> Result<StorageDiagnostics, InitError> {
    let storage_backend = probe_storage(destination)?;
    if storage_backend == StorageBackend::CopyOnWrite {
        probe_full_copy(destination)?;
    }
    Ok(StorageDiagnostics {
        storage_backend,
        native_primitive: native_primitive(storage_backend),
        environment: storage_environment(),
        filesystem: filesystem_for_path(destination),
        full_copy_fallback_verified: true,
        mutable_hard_links_enabled: false,
        isolation_supported: true,
    })
}

fn native_primitive(backend: StorageBackend) -> NativeStoragePrimitive {
    native_primitive_for(backend, current_platform())
}

const fn native_primitive_for(
    backend: StorageBackend,
    platform: NativePlatform,
) -> NativeStoragePrimitive {
    if matches!(backend, StorageBackend::FullCopy) {
        NativeStoragePrimitive::Unavailable
    } else {
        match platform {
            NativePlatform::MacOs => NativeStoragePrimitive::ApfsClone,
            NativePlatform::Linux => NativeStoragePrimitive::LinuxReflink,
            NativePlatform::Windows => NativeStoragePrimitive::WindowsReFsBlockClone,
            NativePlatform::Other => NativeStoragePrimitive::NativeClone,
        }
    }
}

const fn current_platform() -> NativePlatform {
    if cfg!(target_os = "macos") {
        NativePlatform::MacOs
    } else if cfg!(target_os = "linux") {
        NativePlatform::Linux
    } else if cfg!(target_os = "windows") {
        NativePlatform::Windows
    } else {
        NativePlatform::Other
    }
}

#[cfg(target_os = "linux")]
fn storage_environment() -> StorageEnvironment {
    fs::read_to_string("/proc/sys/kernel/osrelease").map_or(StorageEnvironment::Native, |release| {
        storage_environment_from_release(&release)
    })
}

#[cfg(not(target_os = "linux"))]
const fn storage_environment() -> StorageEnvironment {
    StorageEnvironment::Native
}

fn storage_environment_from_release(release: &str) -> StorageEnvironment {
    let release = release.to_ascii_lowercase();
    if release.contains("microsoft") || release.contains("wsl") {
        StorageEnvironment::WindowsSubsystemForLinux
    } else {
        StorageEnvironment::Native
    }
}

#[cfg(target_os = "linux")]
fn filesystem_for_path(path: &Path) -> Option<String> {
    let mountinfo = fs::read_to_string("/proc/self/mountinfo").ok()?;
    filesystem_for_path_from_mountinfo(path, &mountinfo)
}

#[cfg(not(target_os = "linux"))]
fn filesystem_for_path(_path: &Path) -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
fn filesystem_for_path_from_mountinfo(path: &Path, mountinfo: &str) -> Option<String> {
    let mut selected: Option<(usize, String)> = None;
    for line in mountinfo.lines() {
        let Some((mount, filesystem)) = mountinfo_entry(line) else {
            continue;
        };
        if path.starts_with(&mount)
            && selected
                .as_ref()
                .is_none_or(|(length, _)| mount.as_os_str().len() > *length)
        {
            selected = Some((mount.as_os_str().len(), filesystem));
        }
    }
    selected.map(|(_, filesystem)| filesystem)
}

#[cfg(target_os = "linux")]
fn mountinfo_entry(line: &str) -> Option<(PathBuf, String)> {
    let (mount_fields, filesystem_fields) = line.split_once(" - ")?;
    let mount = mount_fields.split_whitespace().nth(4)?;
    let filesystem = filesystem_fields.split_whitespace().next()?.to_owned();
    Some((decode_mountinfo_path(mount), filesystem))
}

#[cfg(target_os = "linux")]
fn decode_mountinfo_path(value: &str) -> PathBuf {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\'
            && index + 3 < bytes.len()
            && bytes[index + 1..=index + 3]
                .iter()
                .all(|byte| matches!(byte, b'0'..=b'7'))
        {
            let value = (bytes[index + 1] - b'0') * 64
                + (bytes[index + 2] - b'0') * 8
                + (bytes[index + 3] - b'0');
            decoded.push(value);
            index += 4;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    PathBuf::from(OsString::from_vec(decoded))
}

impl fmt::Display for DoctorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Project(error) => write!(formatter, "{error}"),
            Self::Probe(error) => write!(formatter, "{error}"),
            Self::FileSystem {
                action,
                path,
                source,
            } => write!(formatter, "could not {action} {}: {source}", path.display()),
            Self::ProbeCleanupFailed {
                probe,
                path,
                source,
            } => write!(
                formatter,
                "{probe}; cleanup could not remove {}: {source}",
                path.display()
            ),
        }
    }
}

impl Error for DoctorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Project(error) => Some(error),
            Self::Probe(error) => Some(error),
            Self::FileSystem { source, .. } => Some(source),
            Self::ProbeCleanupFailed { probe, .. } => Some(probe.as_ref()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        NativePlatform, NativeStoragePrimitive, StorageBackend, StorageEnvironment,
        native_primitive_for, storage_environment_from_release,
    };

    #[cfg(target_os = "linux")]
    use super::filesystem_for_path_from_mountinfo;

    #[test]
    fn wsl_kernel_releases_are_distinguished_from_native_linux() {
        assert_eq!(
            storage_environment_from_release("6.6.87.2-microsoft-standard-WSL2"),
            StorageEnvironment::WindowsSubsystemForLinux
        );
        assert_eq!(
            storage_environment_from_release("4.4.0-19041-Microsoft"),
            StorageEnvironment::WindowsSubsystemForLinux
        );
        assert_eq!(
            storage_environment_from_release("6.8.0-52-generic"),
            StorageEnvironment::Native
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn filesystem_detection_uses_the_most_specific_mount_and_decodes_paths() {
        let mountinfo = concat!(
            "malformed entry that must be ignored\n",
            "24 1 8:1 / / rw,relatime - ext4 /dev/sda rw\n",
            "25 24 0:44 / /mnt/c rw,noatime - 9p C:\\ rw\n",
            "26 24 0:45 / /mnt/c/Dev\\040Drive rw,noatime - virtiofs dev-drive rw\n",
        );

        assert_eq!(
            filesystem_for_path_from_mountinfo(Path::new("/mnt/c/project"), mountinfo),
            Some("9p".to_owned())
        );
        assert_eq!(
            filesystem_for_path_from_mountinfo(Path::new("/mnt/c/Dev Drive/project"), mountinfo),
            Some("virtiofs".to_owned())
        );
        assert_eq!(
            filesystem_for_path_from_mountinfo(Path::new("/tmp/project"), mountinfo),
            Some("ext4".to_owned())
        );
    }

    #[test]
    fn native_primitive_names_every_supported_platform_adapter() {
        assert_eq!(
            native_primitive_for(StorageBackend::CopyOnWrite, NativePlatform::MacOs),
            NativeStoragePrimitive::ApfsClone
        );
        assert_eq!(
            native_primitive_for(StorageBackend::CopyOnWrite, NativePlatform::Linux),
            NativeStoragePrimitive::LinuxReflink
        );
        assert_eq!(
            native_primitive_for(StorageBackend::CopyOnWrite, NativePlatform::Windows),
            NativeStoragePrimitive::WindowsReFsBlockClone
        );
        assert_eq!(
            native_primitive_for(StorageBackend::CopyOnWrite, NativePlatform::Other),
            NativeStoragePrimitive::NativeClone
        );
        assert_eq!(
            native_primitive_for(StorageBackend::FullCopy, NativePlatform::Windows),
            NativeStoragePrimitive::Unavailable
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_copy_on_write_reports_the_refs_block_clone_primitive() {
        assert_eq!(
            super::native_primitive(super::StorageBackend::CopyOnWrite),
            super::NativeStoragePrimitive::WindowsReFsBlockClone
        );
    }
}

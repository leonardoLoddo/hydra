use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

use uuid::Uuid;

use crate::{
    HeadError, InitError, StorageBackend, head,
    init::storage::{probe_full_copy, probe_storage},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeStoragePrimitive {
    ApfsClone,
    LinuxReflink,
    NativeClone,
    Unavailable,
}

#[derive(Debug)]
pub struct StorageDiagnostics {
    pub storage_backend: StorageBackend,
    pub native_primitive: NativeStoragePrimitive,
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
        full_copy_fallback_verified: true,
        mutable_hard_links_enabled: false,
        isolation_supported: true,
    })
}

fn native_primitive(backend: StorageBackend) -> NativeStoragePrimitive {
    if backend == StorageBackend::FullCopy {
        NativeStoragePrimitive::Unavailable
    } else if cfg!(target_os = "macos") {
        NativeStoragePrimitive::ApfsClone
    } else if cfg!(target_os = "linux") {
        NativeStoragePrimitive::LinuxReflink
    } else {
        NativeStoragePrimitive::NativeClone
    }
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

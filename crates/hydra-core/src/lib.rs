//! Core domain behavior for Hydra.

mod init;

pub use init::{InitError, InitializedProject, StorageBackend, initialize};

//! Core domain behavior for Hydra.

mod head;
mod init;

pub use head::{CreateHeadOptions, CreatedHead, HeadError, create_head};
pub use init::{InitError, InitializedProject, StorageBackend, initialize};

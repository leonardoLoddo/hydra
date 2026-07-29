//! Core domain behavior for Hydra.

mod head;
mod init;

pub use head::{
    ChangeCounts, CreateHeadOptions, CreatedHead, HeadError, HeadInspection, HeadSummary,
    ProjectInspection, WorktreeHead, create_head, head_path, inspect_head, inspect_project,
    list_heads,
};
pub use init::{InitError, InitializedProject, StorageBackend, initialize};

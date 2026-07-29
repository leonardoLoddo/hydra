//! Core domain behavior for Hydra.

mod head;
mod init;

pub use head::{
    ChangeCounts, ClosedHead, CreateHeadOptions, CreatedHead, HeadCreationProgress, HeadError,
    HeadInspection, HeadSummary, ProjectInspection, RemoveHeadOptions, RemovedHead, WorktreeHead,
    close_head, create_head, create_head_with_progress, head_path, inspect_head, inspect_project,
    list_heads, remove_head,
};
pub use init::{InitError, InitializedProject, StorageBackend, initialize};

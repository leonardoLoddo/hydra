//! Core domain behavior for Hydra.

mod doctor;
mod head;
mod init;

pub use doctor::{DoctorError, NativeStoragePrimitive, StorageDiagnostics, diagnose_storage};
pub use head::{
    ChangeCounts, CloseOutcome, ClosedHead, CreateHeadOptions, CreatedHead, HeadCreationProgress,
    HeadError, HeadInspection, HeadSummary, OpenedHead, ProjectInspection, RemoveHeadOptions,
    RemovedHead, RepairIssue, RepairPlan, RepairResult, WorktreeHead, apply_repairs, close_head,
    create_head, create_head_with_progress, head_path, inspect_head, inspect_project, list_heads,
    open_head, plan_repairs, remove_head,
};
pub use init::{InitError, InitializedProject, StorageBackend, initialize};

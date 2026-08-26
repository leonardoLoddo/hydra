//! Core domain behavior for Hydra.

mod doctor;
mod head;
mod init;
mod path;

pub use doctor::{
    DoctorError, NativeStoragePrimitive, StorageDiagnostics, StorageEnvironment, diagnose_storage,
};
pub use head::{
    ChangeCounts, CloseOutcome, ClosedHead, CreateHeadOptions, CreatedHead, HeadCreationProgress,
    HeadError, HeadInspection, HeadSummary, IntegrationResult, IntegrationStrategy,
    InventoryRecoveryResult, OpenedHead, PendingCreationRecoveryResult, ProjectInspection,
    RemoveHeadOptions, RemovedHead, RepairIssue, RepairPlan, RepairResult, WorktreeHead,
    apply_abandoned_state_lock_recovery, apply_inventory_recovery, apply_pending_creation_recovery,
    apply_repairs, apply_untracked_head_recovery, close_head, create_head,
    create_head_with_progress, head_path, inspect_head, inspect_project, list_heads, open_head,
    plan_repairs, remove_head,
};
pub use init::{InitError, InitializedProject, StorageBackend, initialize};

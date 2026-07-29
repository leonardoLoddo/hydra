# Architecture Context Router

## Purpose

This router owns Hydra's implemented technical structure: component boundaries,
dependency direction, orchestration of side effects, and the internal workflows
that realize product behavior.

It does not redefine product scope or user-visible guarantees, which remain
owned by the product domain. It also does not own Rust/TDD or commit
conventions, which remain in the development domain.

---

## Routes

| Document | Consult when the task involves | Nature of the document |
|---|---|---|
| [`system-architecture.md`](system-architecture.md) | Cargo workspace structure, crate responsibilities, dependency direction, CLI/core separation, introduction of a new crate, or placement of Git and filesystem behavior | Current structural architecture and rules for evolving component boundaries without premature fragmentation. |
| [`project-initialization.md`](project-initialization.md) | `hydra init`, Git repository discovery, initial configuration or local state, default Heads directory creation, atomic persistence, initialization rollback, or initialization errors | Implemented initialization workflow, persistence sequence, failure behavior, verification contracts, and explicitly recorded gaps against the product specification. |
| [`head-creation.md`](head-creation.md) | `hydra head create`, Head names, base or target refs, private branches, worktree registration, tracked or overlay materialization, Head metadata, state locking, creation rollback, or post-commit cleanup | Implemented Head-creation transaction, CLI/core boundary, Git and filesystem safety rules, verification contracts, and explicitly recorded gaps against the product specification. |
| [`head-inspection.md`](head-inspection.md) | `hydra status`, `hydra head list`, `hydra head status`, `hydra head path`, read-only inventory access, Git status counting, ahead/behind calculation, or consistency diagnostics | Implemented read-only inspection model, CLI output contracts, trust-boundary validation, Git comparison rules, and verification coverage. |
| [`head-removal.md`](head-removal.md) | `hydra head remove`, forced removal, worktree deletion, integrated or preserved private branches, inventory removal, or partial-removal recovery | Implemented protected Head-removal transaction, safety validation, branch recoverability, failure boundaries, and regression coverage. |
| [`head-close.md`](head-close.md) | `hydra head close`, isolated fast-forward or merge integration, target-ref updates, conflict preservation, checked-out targets, or removal after integration | Implemented close transaction, checkout-free integration, compare-and-swap target publication, conflict behavior, and protected-removal composition. |
| [`head-open.md`](head-open.md) | `hydra head open`, configured open adapters, command placeholders, process argument separation, opener failures, or pre-launch worktree validation | Implemented open-command configuration, validated process launch, placeholder expansion, CLI output, and read-only lifecycle boundary. |
| [`repair.md`](repair.md) | `hydra repair`, inventory/worktree reconciliation, stale entries, moved or missing worktrees, untracked Hydra branches, guided recovery, or repair limitations | Implemented read-only repair planning, explicitly confirmed deterministic corrections, report-only inconsistencies, and race-safety boundaries. |
| [`doctor-storage.md`](doctor-storage.md) | `hydra doctor storage`, real Heads-volume capability probes, native clone reporting, full-copy verification, mutable-hard-link policy, diagnostic cleanup, or storage-doctor errors | Implemented storage diagnostic boundary, shared probe behavior, output model, isolation claims, and cleanup contract. |

---

## Consultation Rules

- Consult `system-architecture.md` before adding a crate, moving behavior across
  crates, changing dependency direction, or deciding whether code belongs in
  the CLI or core.
- Consult `project-initialization.md` for every change that can affect the
  behavior or failure state of `hydra init`.
- Consult `head-creation.md` for every change that can affect Head creation,
  overlay confirmation or selection, private branch/worktree setup, Head
  metadata, or creation rollback.
- Consult `head-inspection.md` for every change that affects how existing Heads
  are listed, resolved, compared with Git, summarized, or diagnosed without
  mutation.
- Consult `head-removal.md` for every change that can remove a Head worktree,
  inventory entry, or private branch, including behavior reused by `head
  close`.
- Consult `head-close.md` for every change that integrates a private Head
  branch into its target or composes integration with protected removal.
- Consult `head-open.md` for every change that starts a configured process for
  a Head or changes open-command placeholders and validation.
- Consult `repair.md` for every change that detects or mutates inconsistent
  Hydra inventory, worktree paths, or recoverable branch state.
- Consult `doctor-storage.md` for every change that diagnoses the active
  storage volume or reports clone, fallback, hard-link, and isolation support.
- Consult both documents when initialization work changes a component boundary
  or introduces a new Git, configuration, state, or filesystem component.
- Combine this router with the product router for user-visible behavior and
  safety guarantees.
- Combine this router with the development router for every Rust implementation,
  test, dependency, quality-gate, or commit operation.

Routes are cumulative. Architecture documents describe the implementation of a
contract; they do not weaken requirements owned by the product specification.

---

## Domain Maintenance

Add a new architecture document only when implemented behavior establishes a
stable responsibility that cannot remain clear in an existing leaf document.
Do not create documents for speculative components or planned crate names.

When adding, renaming, moving, or removing a document:

1. update the **Routes** table;
2. update consultation rules when ownership changes;
3. update the parent router at
   [`../hydra-context-router.md`](../hydra-context-router.md);
4. verify all relative links.

Do not store task plans, implementation diaries, or change logs in this domain.

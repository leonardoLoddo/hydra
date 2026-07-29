# System Architecture

## Purpose

This document defines the implemented structural boundaries of Hydra and the
rules for evolving them. It records responsibilities that future contributors
must preserve unless an intentional architecture change updates this document
and its routed dependencies.

The product model and MVP scope remain authoritative in
[`../product/hydra-mvp-context.md`](../product/hydra-mvp-context.md).

---

## Current Workspace

Hydra is a Cargo workspace with two packages:

```text
Hydra/
├── Cargo.toml
├── rust-toolchain.toml
└── crates/
    ├── hydra-cli/
    │   ├── src/
    │   │   ├── inspection.rs
    │   │   ├── main.rs
    │   │   └── output.rs
    │   └── tests/
    │       ├── common/mod.rs
    │       ├── cli_contract.rs
    │       ├── head_create_conflicts.rs
    │       ├── head_create_overlay_failures.rs
    │       ├── head_create_state_failures.rs
    │       ├── head_create_success.rs
    │       ├── head_inspection.rs
    │       ├── init_conflicts.rs
    │       ├── init_git_errors.rs
    │       └── init_success.rs
    └── hydra-core/
        └── src/
            ├── lib.rs
            ├── head.rs
            ├── head/
            │   ├── error.rs
            │   ├── git.rs
            │   ├── inspection.rs
            │   ├── materializer.rs
            │   ├── overlay.rs
            │   ├── persistence.rs
            │   ├── state.rs
            │   └── state/
            │       ├── configuration.rs
            │       └── installation.rs
            ├── init.rs
            └── init/
                ├── artifacts.rs
                ├── configuration.rs
                ├── error.rs
                ├── git.rs
                ├── persistence.rs
                └── storage.rs
```

The root Cargo manifests are authoritative for the exact Rust version, edition,
dependency versions, and lint configuration. This document intentionally does
not duplicate those values.

The product specification describes additional possible crates such as
`hydra-git`, `hydra-materializer`, `hydra-overlays`, and `hydra-config`. They
are not current components. Introduce one only after implemented behavior
establishes a stable boundary that the existing crates cannot represent
clearly.

---

## Dependency Direction

The current dependency graph is:

```text
hydra-cli ──────> hydra-core
     │                 │
     │                 ├── Git process boundary
     │                 └── filesystem and persistence
     │
     └── terminal input/output and exit status
```

`hydra-core` MUST NOT depend on `hydra-cli`. Domain behavior must remain
callable without constructing command-line arguments or capturing terminal
output.

Third-party dependencies are declared at workspace level when shared version
coordination is useful. A crate opts into only the dependencies it uses.

---

## `hydra-cli` Responsibilities

`hydra-cli` is the executable adapter. It owns:

- the public command hierarchy and argument parsing;
- documented command names, positional arguments, flags, and help output;
- conversion from CLI values into calls to `hydra-core`;
- human-readable stdout and stderr;
- process exit status.

The CLI should not own Git discovery, path policy, persistence, rollback, or
other domain decisions. Keeping those operations in the core allows tests and
future interfaces to reuse one implementation.

CLI integration tests execute the compiled `hydra` binary and assert externally
observable behavior. Tests that mutate Git or the filesystem use newly created
temporary repositories.

---

## `hydra-core` Responsibilities

`hydra-core` owns product rules, state-changing workflows, and read-only
inspection independent of the terminal interface. It currently owns project
initialization, Head creation, and Head inspection, including:

- Git repository and common-directory discovery;
- derivation and validation of initialization paths;
- configuration and local-state serialization;
- real storage capability probing on the Heads volume;
- atomic publication of state files;
- rollback of artifacts created by a failed initialization;
- private branch and no-checkout worktree creation;
- tracked and overlay materialization with CoW/copy isolation;
- transactional Head metadata publication and creation rollback;
- validated read-only inventory loading and Head path resolution;
- Git worktree state, change counts, ahead/behind, and consistency diagnostics;
- typed errors with preserved sources for operational failures.

Public core APIs return data or typed errors. They do not print, terminate the
process, or parse CLI syntax.

Expected failures from Git, user paths, existing state, serialization, and the
filesystem return errors rather than panicking.

### Initialization module boundaries

Project initialization is kept inside one public core capability while its
internal responsibilities remain separated:

| Module | Responsibility |
|---|---|
| `init.rs` | Orchestrate initialization and validate derived destinations before mutation |
| `init/git.rs` | Execute Git discovery commands and translate their path output |
| `init/configuration.rs` | Build and serialize shared configuration, local locator, ownership marker, and initial inventory |
| `init/storage.rs` | Probe native CoW support and verify the full-copy fallback |
| `init/persistence.rs` | Sequence filesystem mutations and publish metadata atomically |
| `init/artifacts.rs` | Track exact owned artifacts and perform non-recursive rollback |
| `init/error.rs` | Define and render typed initialization and cleanup failures |

These are private implementation modules, not independent services or crates.
They may depend on one another only through narrow `pub(super)` functions and
types. The public API continues to be re-exported by `hydra-core/src/lib.rs`.

### Head creation module boundaries

Head creation follows the same small-orchestrator rule:

| Module | Responsibility |
|---|---|
| `head.rs` | Validate and orchestrate the complete creation transaction |
| `head/git.rs` | Discover Git state and own ref, branch, index, worktree, and verification commands |
| `head/materializer.rs` | Materialize Git tree entries without a standard checkout |
| `head/overlay.rs` | Expand overlay rules, select safe source files, copy them, and verify content identity |
| `head/state.rs` | Manage the physical inventory transaction and classify commit boundaries |
| `head/state/configuration.rs` | Parse and validate schema-v2 directory policies and shared Head settings |
| `head/state/installation.rs` | Resolve the Git-common locator, verify directory ownership and worktree boundaries, and locate the physical inventory |
| `head/persistence.rs` | Acquire and release the state lock and replace local state atomically |
| `head/error.rs` | Define and render typed creation, rollback, and post-commit cleanup failures |

The terminal confirmation remains in `hydra-cli`; the core exposes the
confirmation requirement as data and recomputes the overlay plan after
confirmation. Detailed workflow ownership is documented in
[`head-creation.md`](head-creation.md).

### Head inspection module boundaries

| Module | Responsibility |
|---|---|
| `head/inspection.rs` | Compose validated inventory metadata, filesystem presence, Git refs, worktree registration, changes, and ahead/behind into public read-only models |
| `head/git.rs` | Execute and parse the Git queries shared by creation and inspection |
| `head/state.rs` | Load the versioned inventory without taking the mutation lock and expose metadata through narrow internal accessors |
| `hydra-cli/src/inspection.rs` | Render project summaries, detailed Head status, ordered names, paths, and command exit status |
| `hydra-cli/src/output.rs` | Neutralize control characters for human terminal output while allowing command-specific raw machine output |

Inspection reuses the same configuration, locator, ownership, and directory
policy validation as creation. It does not acquire `heads.json.lock`, write
state, or repair inconsistencies. Detailed behavior is documented in
[`head-inspection.md`](head-inspection.md).

---

## Git and Process Boundary

Hydra currently integrates with Git through `std::process::Command`.

Every Git invocation MUST:

- pass the executable and each argument separately;
- avoid shell command construction and interpolation;
- set the repository context explicitly with `git -C <path>` when operating
  outside a Head;
- check the exit status before consuming output;
- validate output before converting it into paths or state.

If Git integration becomes large enough to justify a separate `hydra-git`
crate, the new crate should own only the Git adapter and Git-specific data
translation. Product decisions must remain in `hydra-core`.

---

## Filesystem and Persistence Boundary

Filesystem mutations belong to core workflows or to a future narrow adapter
introduced by demonstrated platform needs.

Multi-step workflows must:

1. validate all predictable conflicts before mutation;
2. serialize fallible in-memory data before creating filesystem artifacts;
3. publish individual state files atomically;
4. track which artifacts were created by the current operation;
5. remove only those owned artifacts during rollback;
6. preserve pre-existing or non-empty directories rather than deleting
   ambiguous user data.

Platform-specific storage primitives must remain behind narrow interfaces.
They must not leak platform conditions into CLI parsing or general Head
lifecycle rules.

The current storage boundary uses the safe `reflink-copy` API to reach APFS
clone and Linux reflink primitives. Hydra verifies the resulting bytes and
always verifies a full-copy fallback when cloning is unavailable. Native
results from one platform do not establish support on another.

---

## Evolution Rules

Create or split a crate only when all of the following are true:

1. the responsibility exists in production code;
2. its boundary is stable enough to name;
3. moving it reduces coupling or enables required platform isolation;
4. representative consumers and regression tests can protect the move;
5. the owning architecture documents and routers are updated in the same
   change.

Do not create a crate solely because it appears in the recommended future
layout. Prefer modules inside `hydra-core` until a stronger boundary emerges.

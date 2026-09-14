# System Architecture

**Status:** current
**Scope:** Cargo workspace boundaries, crate responsibilities, and dependency direction
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read this leaf before adding a crate, moving responsibilities across crates,
changing dependency direction, choosing persistence technology, or deciding
whether behavior belongs in the CLI or core.

## Inherited defaults

Load these product contracts before interpreting the implementation rules:

- [hydra-mvp-context](../product/hydra-mvp-context.md)

The local rules extend those contracts with implementation constraints. Safety
summaries retain local visibility; the linked product rules own product policy.

## Purpose

This document defines the implemented structural boundaries of Hydra and the
rules for evolving them. It records responsibilities that future contributors
must preserve unless an intentional architecture change updates this document
and its routed dependencies.

The product model and MVP scope remain authoritative in
[hydra-mvp-context.md](../product/hydra-mvp-context.md).

---

## Workspace boundary

Hydra has two Cargo packages, `hydra-cli` and `hydra-core`. The root manifests
own exact workspace membership, Rust version, edition, dependencies, and lints.
Do not maintain a second file inventory or copy version numbers here.

Additional Git, materialization, overlay, or configuration crates are not current
components. Introduce one only after a stable implemented boundary cannot be
represented clearly by private modules in the existing crates.

## Dependency Direction

The current dependency graph is:

```text
hydra-cli ──────> hydra-core
     │                 │
     │                 ├── Git process boundary
     │                 └── filesystem and persistence
     │
     ├── terminal input/output and exit status
     └── provider-specific skill distribution adapters
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

For unsafe overlay symlinks, the CLI owns only presentation and confirmation:
it renders the relative paths returned by the core and translates an explicit
answer into retry authorization. Selection of the exact exclusion rules,
validation, atomic `.hydra.json` replacement, and the ordering before Git
mutation remain core responsibilities.

The private `head_create.rs` CLI module owns orchestration of the interactive
creation retries, progress rendering, confirmation prompts, and the final
success or error output. Domain planning, filesystem mutation, rollback, and
recovery remain in `hydra-core`; `main.rs` only dispatches the parsed creation
arguments to this adapter.

The private `repair.rs` CLI module owns repair orchestration and conversion of
the selected core repair result into terminal output and exit status. Its
`repair/presentation.rs` child owns issue rendering and confirmation prompts.
`main.rs` retains only command definition and dispatch; repair classification,
validation, and mutation remain in `hydra-core`.

The private `guidance.rs` CLI module maps typed core and skill errors to one
actionable next step. It owns presentation only: error classification and safe
recovery boundaries remain in the originating domain type. Command adapters use
this shared mapping so equivalent failures do not offer conflicting recovery
advice.

The private `skill.rs` CLI module is a narrow distribution adapter rather than
Hydra repository-domain behavior. It resolves each supported provider's
documented personal skill location, renders default-negative confirmations,
stages the canonical embedded skill, and owns its provider-specific provenance
manifest. Provider destinations and exact ownership checks are canonical in
[../development/release-distribution.md](../development/release-distribution.md);
load that contract for adapter changes. Unknown or locally modified installed
content remains protected. It does not read or mutate a Hydra
project's Git repository, Heads directory, or local metadata, so this
host-specific lifecycle does not belong in `hydra-core`.

CLI integration tests execute the compiled `hydra` binary and assert externally
observable behavior. Tests that mutate Git or the filesystem use newly created
temporary repositories.

---

## `hydra-core` Responsibilities

`hydra-core` owns product rules, state-changing workflows, and read-only
inspection independent of the terminal interface. It currently owns project
initialization, Head creation, and Head inspection, including:

- Git repository and common-directory discovery;
- canonical parent-project resolution for lifecycle commands invoked from any
  managed Head;
- derivation and validation of initialization paths;
- configuration and local-state serialization;
- real storage capability probing on the Heads volume;
- explicit storage diagnostics with native and fallback verification;
- atomic publication of state files;
- rollback of artifacts created by a failed initialization;
- private branch and no-checkout worktree creation;
- tracked and overlay materialization with CoW/copy isolation;
- transactional Head metadata publication and creation rollback;
- protected Head removal with recoverable private-branch preservation;
- parent-worktree Head integration through a foreground native Git merge, with
  validated conflict continuation and abort handling;
- validated execution of configured Head-close adapters with optional
  protected removal;
- validated execution of configured Head-open adapters without a shell;
- guided reconciliation of inventory, worktree paths, and private branches;
- validated read-only inventory loading and Head path resolution;
- Git worktree state, change counts, ahead/behind, and consistency diagnostics;
- typed errors with preserved sources for operational failures.

Public core APIs return data or typed errors. They do not print, terminate the
process, or parse CLI syntax.

Expected failures from Git, user paths, existing state, serialization, and the
filesystem return errors rather than panicking.

### Side-effect ownership

Keep orchestration separate from side-effect boundaries using private modules
and narrow `pub(super)` APIs. Public operations remain exported through
`crates/hydra-core/src/lib.rs`; do not widen visibility merely to move files.

- Initialization separates configuration serialization, Git discovery, storage
  probing, atomic persistence, exact artifact rollback, and empty-installation
  recovery. The initialization workflow owns mutation order.
- Creation separates Git refs and worktrees, validated Git protocol decoding,
  tracked materialization, overlay selection and materialization, state
  transactions, recovery evidence, and errors. Persistent blob reads and bounded
  overlay hashing belong behind those materialization boundaries.
- Inspection composes read-only state and Git observations. Terminal escaping
  remains in the CLI; state validation remains in core.
- Repair separates read-only planning from revalidated application. Prompt
  presentation belongs to the CLI, never the repair planner.
- Open and close share command-template validation; close composes protected
  removal rather than implementing another deletion policy.
- Doctor reuses the initialization storage adapter. It owns diagnostic-directory
  lifecycle and combined probe/cleanup errors, not another clone implementation.

Creation confirmation requests and progress are typed core data. The CLI presents
them and supplies explicit retry authorization. The core recomputes plans and
owns policy changes, mutations, rollback, and recovery. Informational progress
observers MUST NOT interrupt transactions.

Select workflow details through the [architecture router](ROUTER.md). Do not copy
those transaction rules into this structural boundary document.

## Git and Process Boundary

Hydra currently integrates with Git through `std::process::Command`.

Every Git invocation MUST:

- pass the executable and each argument separately;
- avoid shell command construction and interpolation;
- set the repository context explicitly with `git -C <path>` when operating
  outside a Head;
- check the exit status before consuming output;
- validate output before converting it into paths or state.

High-cardinality process performance and protocol validation are owned by
[materialization.md](materialization.md). Load that leaf when changing shared Git
batching or blob readers; path safety and error propagation remain mandatory.

If Git integration becomes large enough to justify a separate `hydra-git`
crate, the new crate should own only the Git adapter and Git-specific data
translation. Product decisions must remain in `hydra-core`.

---

## Persistence technology decision

JSON files with atomic publication are sufficient for the core MVP baseline.
Do not add SQLite or another persistence runtime merely to represent local Head
state. Additional native dependencies are not required by that baseline except
for the minimal adapters needed by platform CoW primitives. A new requirement
must justify revisiting this decision through the Development dependency rules.

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
clone, Linux `FICLONE`, and Windows ReFS block-clone primitives. Hydra verifies the
resulting bytes and always verifies a full-copy fallback when cloning is
unavailable. Linux diagnostics identify WSL and the filesystem owning the
actual Heads volume, but neither label can override the real clone probe.
Native results from one platform or volume do not establish support on
another.

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

## Evidence and verification

Inspect the root Cargo manifests, `crates/hydra-core/src/lib.rs`,
`crates/hydra-cli/src/main.rs`, and the specific workflow modules selected by
the architecture router. Verify core public APIs return data or typed errors,
CLI policy decisions stay in core, and configured host skill distribution does
not touch project state. For structural changes, run the complete before/after
baseline and quality gates required by the Development structural-refactoring
route; representative consumers must preserve public and persisted contracts.

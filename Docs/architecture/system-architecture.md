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
    │   ├── src/main.rs
    │   └── tests/cli.rs
    └── hydra-core/
        └── src/
            ├── lib.rs
            └── init.rs
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

`hydra-core` owns product rules and state-changing workflows independent of the
terminal interface. It currently owns project initialization, including:

- Git repository and common-directory discovery;
- derivation and validation of initialization paths;
- configuration and local-state serialization;
- atomic publication of state files;
- rollback of artifacts created by a failed initialization;
- typed errors with preserved sources for operational failures.

Public core APIs return data or typed errors. They do not print, terminate the
process, or parse CLI syntax.

Expected failures from Git, user paths, existing state, serialization, and the
filesystem return errors rather than panicking.

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

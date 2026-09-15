# Storage Doctor

**Status:** current
**Scope:** `hydra doctor storage` capability probing and reporting
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read this leaf when a task changes storage capability detection, native clone
reporting, full-copy fallback, isolation claims, or diagnostic cleanup.

Skip when neither the stated workflow nor a shared boundary it depends on can
be affected. A nearby command name alone does not select this leaf.

## Inherited defaults

Load these product contracts before interpreting the implementation rules:

- [storage-and-platforms](../product/storage-and-platforms.md)

The local rules extend those contracts with implementation constraints. Safety
summaries retain local visibility; the linked product rules own product policy.

## Purpose

This document defines the implemented storage diagnostic:

```text
hydra doctor storage
```

The command verifies the actual volume that owns the initialized project's
Heads directory. Product requirements remain authoritative in
[storage-and-platforms.md](../product/storage-and-platforms.md).

---

## Validation and Probe Location

Diagnostics use the same schema-v2 configuration, Git-common locator,
directory marker, ownership, and directory-policy validation as Head
inspection. The command requires an initialized and internally consistent Hydra
installation.

After validation, Hydra creates one uniquely named diagnostic directory as a
direct child of the managed Heads directory. Every source and target probe file
is created exclusively inside that directory. Running a probe on the host
operating system or another temporary volume would not prove the capability of
the real Heads destination.

The diagnostic does not acquire `heads.json.lock`, read or modify Head content,
or mutate configuration, inventory, refs, and worktrees.

---

## Capability Tests

The native test uses the same `reflink-copy` adapter as initialization and Head
materialization:

1. create and synchronize a source file;
2. attempt the platform clone primitive;
3. read the target and verify exact contents;
4. remove both files.

When the native clone succeeds, Hydra separately forces the exclusive
full-copy path and verifies its bytes. When native cloning is unavailable, the
normal probe already exercises and verifies that fallback.

The reported primitive describes the adapter whose real clone attempt
succeeded:

- `APFS clone` on macOS;
- `Linux reflink` on Linux;
- `Windows ReFS block clone` when the Windows adapter succeeds on a compatible
  ReFS or Dev Drive volume;
- `native clone` on another supported target;
- `unavailable` when the verified backend is full copy.

On Linux, diagnostics also resolve the filesystem that owns the probe path
from `/proc/self/mountinfo`, choosing the most specific enclosing mount and
decoding kernel path escapes. Malformed unrelated entries do not suppress a
valid match. The kernel release identifies Windows Subsystem for Linux without
changing the capability decision: WSL still receives `copy-on-write` only when
the real `FICLONE` probe succeeds.

Hydra does not use mutable hard links as a storage fallback. Isolation is
reported as supported only after either copy-on-write plus fallback
verification or the isolated full-copy probe has completed successfully.

---

## Output Contract

A successful report contains:

```text
Storage backend: copy-on-write
Native primitive: APFS clone
Environment: native
Filesystem: unknown
Fallback: full copy (verified)
Mutable hard links: disabled
Isolation: supported
```

`Storage backend: full copy` and `Native primitive: unavailable` are used when
the native attempt fails safely. Linux reports its resolved filesystem;
non-Linux platforms currently report `unknown`. A full-copy result under WSL
prints the maintained `Docs/user/wsl-copy-on-write.md` URL; native Windows
prints `Docs/user/windows-copy-on-write.md`. Initialization and Head creation
use the same platform-specific URLs. Guidance is informational and never
substitutes an unverified backend.

With `--json`, the same completed probe emits one versioned object containing
`storageBackend`, `nativePrimitive`, `environment`, `filesystem`,
`copyOnWriteGuidance`, `fullCopyFallbackVerified`,
`mutableHardLinksEnabled`, and `isolationSupported`. The root contains
`schemaVersion: 1`. Enum-like values use stable camel-case identifiers such as
`copyOnWrite`, `fullCopy`, `linuxReflink`, and
`windowsSubsystemForLinux`. Unavailable filesystem and guidance values are
`null`. JSON selection does not change the probe, cleanup, or failure contract.

---

## Cleanup and Errors

Hydra removes each probe file and then the diagnostic directory. A probe or
cleanup failure is a failed command and identifies the affected path. If both
the probe and final directory cleanup fail, the error preserves the original
probe failure and reports the remaining diagnostic directory.

The command never recursively removes the diagnostic directory. An unexpected
entry therefore remains visible rather than risking deletion of ambiguous
content.

---

## Verification Contract

Implementation evidence: `crates/hydra-core/src/doctor.rs`. CLI integration evidence: `doctor_storage`
test targets under `crates/hydra-cli/tests/`. Run the affected targets with
`cargo test -p hydra-cli --test <target>` and inspect the observable results below.
A listed test contract is not evidence that every platform passed in this task.

CLI integration tests on the actual test volume prove:

- all required diagnostic lines are emitted after a successful real probe;
- WSL kernel releases and native Linux releases are classified independently;
- Linux mount resolution uses the most specific valid mount and preserves
  escaped mount paths;
- every platform adapter has a distinct reported primitive, including Windows
  block cloning;
- native Windows full-copy output links to the maintained setup guide, while a
  successful Windows block clone does not print fallback guidance;
- WSL full-copy output links to its Linux reflink setup guide;
- the Heads directory contains exactly the same entries before and after;
- no Hydra mutation lock is created;
- versioned JSON contains the same capability result and leaves the same clean state;
- an uninitialized Git repository is rejected without Hydra artifacts;
- nested command help documents the real Heads-volume test.

The shared storage adapter retains unit coverage for forced full-copy
verification, exact byte comparison, cleanup, and cleanup-failure diagnostics.

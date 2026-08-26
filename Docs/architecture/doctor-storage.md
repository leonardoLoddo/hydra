# Storage Doctor

## Purpose

This document defines the implemented storage diagnostic:

```text
hydra doctor storage
```

The command verifies the actual volume that owns the initialized project's
Heads directory. Product requirements remain authoritative in
[`../product/hydra-mvp-context.md`](../product/hydra-mvp-context.md).

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
- `Windows block clone` when the Windows adapter succeeds on a compatible
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
adds guidance to place both the project and its sibling Heads directory on a
reflink-capable Linux filesystem, for example XFS when the running WSL kernel
provides it. The guidance is informational and never substitutes an unverified
backend.

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

CLI integration tests on the actual test volume prove:

- all required diagnostic lines are emitted after a successful real probe;
- WSL kernel releases and native Linux releases are classified independently;
- Linux mount resolution uses the most specific valid mount and preserves
  escaped mount paths;
- every platform adapter has a distinct reported primitive, including Windows
  block cloning;
- the Heads directory contains exactly the same entries before and after;
- no Hydra mutation lock is created;
- an uninitialized Git repository is rejected without Hydra artifacts;
- nested command help documents the real Heads-volume test.

The shared storage adapter retains unit coverage for forced full-copy
verification, exact byte comparison, cleanup, and cleanup-failure diagnostics.

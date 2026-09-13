# Storage and Platform Contract

**Status:** current
**Scope:** materialization guarantees, content identity, native backends, and platform limits
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read for materialization, CoW, copy fallback, storage diagnosis, performance,
symlinks, submodules, or supported platform changes. Skip for unrelated CLI wording.

## Inherited defaults

Load Head isolation and recoverability from
[hydra-mvp-context.md](hydra-mvp-context.md#default-head-isolation-and-recoverability).
The following rules extend that default with materialization requirements.

## Materialization boundary

Git worktrees provide independent Git state while sharing the object database.
A separate Materializer creates normal visible files. Hydra SHOULD avoid a full
standard checkout before writing those files; the implemented workflow registers
a worktree without checkout and initializes its index separately. If a platform
requires another sequence, its observable Git result MUST remain equivalent.
Hydra MUST NOT duplicate the Git object database for each Head. Its space-efficiency
goal is that native CoW initially allocates mainly differences while each Head
still exposes complete normal files; full copy remains an explicit safe fallback.

CoW is an optimization, while write isolation is mandatory. Hydra MUST use native
CoW when supported by the selected storage policy and actual volume, or a safe
exclusive full copy. It MUST NOT use mutable hard links, patch reconstruction,
or an editor watcher as a substitute for filesystem write isolation.

Tracked content is identified by the Git blob expected at `baseCommit`.
Overlay identity is computed from the selected source bytes. Reuse MUST validate
content rather than trusting names, sizes, or timestamps. A Head MUST NOT retain a
writable dependency on a particular source path. The content-reuse model permits a
verified identical source in the parent or another Head; otherwise tracked content
comes from Git and overlays from their selected source. The current cross-Head
search limitation below does not revoke that model. A source changed after
a verified CoW clone cannot invalidate the already isolated destination.

Current tracked-source reuse requires the parent project's complete tracked state
to match `baseCommit`. Any tracked difference disables that fast path for the
whole pass. Blob fallback restores exact committed content. Cross-Head matching
and a persistent content cache are not implemented; the broader content-reuse
model does not justify claiming those optimizations are available.

High-cardinality hashing and blob reads MUST use bounded batches or persistent
Git streams. Preserve deterministic ordering, arbitrary-path handling, executable
permissions, content verification, and propagated errors. Performance changes
MUST NOT weaken those contracts.

## Platforms and limits

| Environment | Native primitive | Safe fallback |
|---|---|---|
| macOS on compatible APFS | native file clone | full copy |
| Linux, including WSL 2, on compatible volumes | `FICLONE` reflink | full copy |
| Native Windows x86-64 on compatible ReFS or Dev Drive | `FSCTL_DUPLICATE_EXTENTS_TO_FILE` block clone | full copy |

Probe the actual Heads destination and actual overlay sources. Operating-system
labels and one successful file probe do not prove every file can be cloned.
Cross-volume operations can require full copy. `storage.mode: "copy"` explicitly
forces copy even if the platform supports cloning.

WSL 2 uses the Linux backend. Its default ext4 root and DrvFs/9p mounts may reject
reflinks. A separate Linux volume is accepted as CoW-capable only after a real
probe succeeds. Hydra MUST NOT create, format, mount, resize, or relocate volumes
as an implicit setup action. WSL 1 is outside supported scope.

Native Windows uses Git for Windows through Git Bash. NTFS and unsupported
volumes use isolated full copy. Tracked and overlay symlink materialization is
unsupported on Windows; Unix support does not establish Windows support.
Gitlink entries create empty directories only. Hydra does not implicitly fetch
or initialize submodule contents or claim isolated ready-to-use submodules.

## Storage diagnosis

`hydra doctor storage` MUST safely probe the initialized project's real Heads
volume and report backend, native primitive, environment, filesystem, verified
fallback, disabled mutable hard links, and isolation outcome. Linux filesystem
and WSL identification refine diagnostics but cannot override probe results.
Non-Linux filesystem reporting currently remains `unknown`.

Full-copy results on WSL and native Windows link the maintained platform setup
guide. Successful CoW does not print fallback guidance. Guidance is information,
not proof or permission to modify a volume. Probe and cleanup failures remain
command failures with retained diagnostics; unexpected files are not recursively
removed.

Logical file size and exclusive physical allocation are distinct. Do not claim
exclusive-space measurements unless the backend can measure them reliably.

## Evidence and verification

Inspect `crates/hydra-core/src/init/storage.rs`,
`crates/hydra-core/src/head/materializer.rs`, and
`crates/hydra-core/src/doctor.rs`. Run native storage unit tests and CLI
`doctor_storage` and `head_create_success` tests on the real test volume, including
forced copy and independent writes after creation. Use `head_create_performance`
only for performance work; its ignored release fixture measures rather than
asserting a machine-independent duration. Report the platform actually exercised.
Detailed diagnostic lifecycle is in
[../architecture/doctor-storage.md](../architecture/doctor-storage.md).

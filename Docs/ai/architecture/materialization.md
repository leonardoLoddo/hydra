# Tracked and Overlay Materialization

**Status:** current
**Scope:** committed-content materialization, overlay planning, content identity, and Git batching
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read when changing tracked-content reuse, Git blob streaming, overlay matching,
hashing, per-file CoW probes, executable permissions, symlinks, or materialization
performance. Skip for creation naming, ref selection, terminal wording, or state
publication changes that cannot affect file content.

## Inherited defaults

Load the storage and isolation rules in
[../product/storage-and-platforms.md](../product/storage-and-platforms.md) and
the overlay selection and confirmation rules in
[../product/configuration-and-overlays.md](../product/configuration-and-overlays.md).
The mechanics below extend those rules. Tracked content comes from `baseCommit`;
overlays come from the canonical parent. Mutable hard links remain forbidden.

## Tracked Materialization

The expected tracked entries and blob identities come from:

```text
git ls-tree -r -z --full-tree <baseCommit>
```

Hydra reads this tree once during preparation and retains the validated entries
for materialization and overlay-collision checks.

Before creating the worktree, Hydra checks whether the source working tree's
tracked state matches `baseCommit`:

```text
git diff --quiet --no-ext-diff <baseCommit> --
```

When it matches, existing regular working files are safe content sources for
that exact commit. Hydra validates that each source is a regular file below
the canonical repository root and attempts a direct CoW clone into the Head.
This both avoids redundant Git decompression and shares the source file's
physical blocks. Untracked files do not disable the fast path. A tracked
change disables it for the complete pass, so uncommitted tracked edits are
never used as the starting content of a new Head. Missing or non-clonable
sources fall back to the blob path below, and final clean-worktree verification
detects a concurrent source change.

Blob fallback uses one lazy, persistent:

```text
git cat-file --batch
```

process for the complete materialization pass rather than one Git process per
entry. Every request is a validated full SHA-1 or SHA-256 object ID. Each
response must echo that ID, declare type `blob`, provide a valid size, contain
exactly that many payload bytes, and end with the protocol newline. Regular
payloads are streamed with a fixed-size buffer; tracked symlink payloads use
the same reader without altering their bytes. The child error stream is
drained with bounded capture, unsuccessful exits are reported, and unfinished
readers terminate and wait for their child defensively.

For a regular entry that needs blob fallback, Hydra streams into a uniquely
named temporary file in the Heads directory and:

1. creates parent directories;
2. attempts a native CoW clone from the temporary blob in `auto` mode;
3. uses an exclusive full copy when cloning is unavailable or `copy` mode was
   selected;
4. synchronizes copied bytes;
5. applies the executable bit represented by Git mode;
6. removes the exact temporary blob.

Mutable hard links are never used.

On Unix, tracked Git symlinks are recreated from their blob payload. On
non-Unix platforms they currently fail as unsupported tracked entries.
Submodule entries create their worktree directory but do not initialize or
fetch submodule content.

Every Git tree path must consist only of normal relative components. Unknown
Git modes and unsafe paths abort creation.

---

## Overlay Planning and Materialization

Overlay files always come from the canonical parent project, including when
the command runs from a managed Head. Tracked files come from `baseCommit`.

The planner:

1. reads `overlay.copy` in order;
2. expands `... <relative-file>` in place;
3. applies Gitignore matching semantics, including negation and precedence;
4. walks only existing entries below the canonical repository root;
5. records each selected entry's logical size and any symlink target;
6. sorts selected relative paths for deterministic materialization;
7. computes regular-file identities with bounded
   `git hash-object --no-filters --` argument batches executed by at most eight
   workers, then restores the original path order;
8. retries an argument batch as smaller ordered halves if the operating system
   reports an argument-list limit;
9. in `auto` mode, probes every regular-file source against the Heads volume
   and records the exact files that need full-copy fallback; in `copy` mode,
   marks every regular overlay for the same confirmation without probing.

An absent expanded rules file contributes no rules. An existing expanded file
must be a regular file at a safe relative path.

Overlay protection rejects:

- `.git` and everything below it;
- special files;
- absolute, broken, or escaping overlay symlinks;
- a selected path that would overwrite a tracked entry;
- an included rules path that is absolute or contains non-normal components;
- a source that no longer resolves inside the repository at materialization
  time.

Initial planning reports all selected unsafe symlinks together so the CLI can
offer their exact persistent exclusions. If that repair is not explicitly
authorized, the protection remains a rejection and no Head artifact is
created. Safe relative symlinks continue through normal materialization and
are never proposed for exclusion.

Parent directories are deduplicated and created before regular-file
materialization. Each source is then revalidated immediately before use. Hydra
uses the same CoW-first, exclusive-copy fallback as tracked regular files, but
performs the fallback only after explicit confirmation. It preserves
permissions, then hashes every materialized destination in bounded parallel
batches and compares it with the identity captured during planning. A source
change that affects the copied payload therefore aborts instead of publishing
a partial Head. A later source removal or edit does not invalidate a
destination that already matches the planned identity and has been isolated by
CoW.

On Unix, a selected symlink is accepted only when its stored target is relative
and its resolved source remains inside the canonical project root. Regular
overlay files are materialized first; Hydra then recreates the symlink with the
same target text instead of dereferencing it. It re-reads the source target
immediately before creation and verifies afterward that the materialized link
resolves inside the canonical Head root. This supports dependency launchers
such as `node_modules/.bin` and `vendor/bin` without linking a Head back to the
source workspace. Symlinks remain unsupported on non-Unix platforms.

The final `git status --porcelain` must be empty. This proves that tracked
materialization matches the index and that selected overlays remain ignored by
the effective Git rules.

## Transaction boundary

This leaf owns file-content mechanics. The caller owns validation, authorized
configuration changes, branch/worktree creation, final state publication, and
rollback through [head-creation.md](head-creation.md). Load that workflow when
changing failure propagation or mutation ordering; it is not an inherited default
for isolated batching or matching changes.

## Evidence and verification

Inspect `crates/hydra-core/src/head/materializer.rs`,
`crates/hydra-core/src/head/materializer/blob_batch.rs`,
`crates/hydra-core/src/head/overlay.rs`, and its `hash.rs` and
`materialization.rs` child modules. Run affected core unit tests and CLI
`head_create_success` and `head_create_overlay_failures` targets. Verify exact
committed bytes, content identity, permission and symlink preservation, ordered
hash results, invalid blob responses, safe copy fallback, source isolation, and
rollback after content changes. Inspect actual Git status and filesystem results.
For batching changes, also exercise existing tracked and overlay consumers.
Performance measurement uses the ignored `head_create_performance` release test;
do not replace correctness assertions with timing evidence.

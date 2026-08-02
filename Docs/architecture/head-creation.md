# Head Creation

## Purpose

This document defines how the current implementation realizes:

```text
hydra head create <name> [--from <ref>] [--target <ref>]
```

It owns the implemented orchestration, Git and filesystem boundaries,
materialization rules, state transaction, and rollback behavior. The intended
user-visible contract remains authoritative in
[`../product/hydra-mvp-context.md`](../product/hydra-mvp-context.md).

An implementation gap recorded here does not relax the product requirement.

---

## CLI Contract

`head create` is a nested command. `name` is required; `--from` and `--target`
are optional.

If `--from` is absent, Hydra resolves `HEAD` in the canonical parent project.
This remains true when the command is invoked from a managed Head: that Head's
private branch and working files do not become implicit creation inputs. The
core returns the created Head path and effective aggregate storage backend.
The CLI prints them only after the worktree and local state have been committed:

```text
New Head successfully created at <absolute-path>
Storage backend: copy-on-write
```

When `stdout` is an interactive Unix terminal, only the path is wrapped in an
OSC 8 hyperlink targeting a `file://` URI. Bytes that are unsafe in a URI are
percent-encoded, and control characters in the visible label are rendered as
textual escapes. A terminal without OSC 8 support still displays the label.
When `stdout` is a pipe or file, Hydra emits no control sequences and preserves
the plain-text message exactly.

The core emits typed, coarse-grained progress events for overlay planning,
tracked materialization, and overlay materialization. The CLI renders one
short line per active phase to `stderr` only when `stderr` is an interactive
terminal:

```text
Planning overlays...
Materializing <count> tracked entries...
Materializing <count> overlay entries...
```

Redirected and captured `stderr` remains free of progress output. Events are
phase-level rather than per-entry, so reporting overhead does not grow with
the number of files. Progress is informational: if an observer panics while
unwinding is available, the core disables that observer instead of allowing it
to interrupt the creation transaction.

The backend line is `Storage backend: full copy` when any materialized regular
file required the safe copy fallback.

For every non-empty overlay plan, successful creation also prints its logical
size and file count:

```text
Overlay: <count> file(s), <bytes> byte(s)
```

This summary is informational and never causes a prompt by itself. During
planning, the core attempts a temporary reflink from every real regular-file
overlay source into one candidate path inside a uniquely created probe
directory owned by the operation. It removes that exact candidate after every
attempt, removes the empty probe directory afterward, and never treats one
file's success as proof that another file can be cloned. Symlinks do not
require a copy probe. When every required probe succeeds, creation proceeds
without terminal input.

The CLI owns terminal interaction. When the core reports that one or more
overlay files require full-copy fallback, it prints only that fallback
subset's logical cost and requests confirmation:

```text
Full copy required: <count> file(s), <bytes> byte(s)
Continue? [y/N]
```

Only `y` and `yes`, compared case-insensitively after trimming whitespace,
confirm the fallback. EOF, an empty response, and every other value cancel it
with a non-zero exit status. Cancellation occurs before branch, worktree, or
state creation.

Unsafe overlay symlinks use a separate explicit repair prompt. Planning
collects every selected symlink that is absolute, broken, escaping, or
unsupported on the current platform and returns relative paths in
deterministic order. The CLI renders:

```text
Unsafe overlay symlinks:
  links/escape
  public/storage
Exclude them and update .hydra.json? [y/N]
```

Only `y` and `yes` authorize the update. A negative answer or EOF leaves
`.hydra.json`, Git, the Heads directory, and the inventory unchanged. On a
confirmed retry, the core converts each relative path into a literal,
root-anchored Gitignore negation such as `!/public/storage`, appends the rules
to `overlay.copy` so last-match-wins semantics exclude those entries, replaces
`.hydra.json` atomically, and replans. Metacharacters and spaces are escaped;
paths that cannot be represented safely in the versioned JSON rules remain
errors. The resulting `.hydra.json` modification is intentionally visible to
Git and must be reviewed and committed by the user.
Serialization retains strict rejection of every unknown top-level field,
including the unavailable `$schema` editor annotation.

This repair applies only to symlinks rejected during initial overlay planning.
Tracked collisions, special files, unsafe included-rule paths, and concurrent
changes during materialization remain hard errors. A symlink that becomes
unsafe after Git mutation begins also remains an error and triggers normal
rollback rather than another configuration prompt.

The core API remains independent of stdin, stdout, and process exit status. A
caller confirms a previously presented full-copy fallback through
`CreateHeadOptions::confirmed_full_copy`; the core recomputes the plan on the
confirmed call rather than trusting stale counts. If a reflink unexpectedly
fails after a successful probe, the unconfirmed operation rolls back and
returns the same confirmation requirement instead of silently duplicating the
file.

Command help follows Git's concise vocabulary and structure: it identifies the
outcome, displays usage and argument semantics, states meaningful defaults such
as `HEAD`, and includes copyable examples. Help only advertises behavior
implemented by the current binary. Both `hydra --help` and
`hydra head --help` include a `Command syntax` index containing the complete
`hydra head create <NAME> [--from <REF>] [--target <BRANCH>]` invocation, so a
nested executable command is visible without traversing each help level.

---

## Name, Base, and Target Resolution

The current Head name grammar is one filesystem-safe component:

- the first character is ASCII alphanumeric;
- remaining characters are ASCII alphanumeric, `.`, `-`, or `_`;
- `..` is forbidden;
- a case-insensitive `.lock` extension is forbidden.

The configured `branchPrefix` is prepended to the name. Git then validates the
complete private branch name. Hydra refuses a duplicate state entry,
pre-existing destination, or pre-existing private branch before mutation.

The base is resolved in the canonical parent repository to both:

- `baseCommit`, using `<from>^{commit}`;
- `baseRef`, normalized to its full symbolic ref when one exists.

If `--target` is present, it must resolve to a local branch and is stored as a
full `refs/heads/...` ref. Without `--target`, a base that resolves to a local
branch becomes the target. A detached commit or other non-local base requires
an explicit target.

User-controlled Git values are passed as individual process arguments. Commit
resolution uses Git's end-of-options handling, and raw refs beginning with `-`
are rejected before symbolic normalization.

---

## Configuration and State Boundary

Head creation reads the parent project's current configuration and shared local
state, regardless of which managed Head invoked the command:

| Data | Path |
|---|---|
| Shared configuration | `<repository-root>/.hydra.json` |
| Local project locator | `<git-common-dir>/hydra/project.json` |
| Directory ownership marker | `<heads-directory>/.hydra/directory.json` |
| Local Head inventory | `<heads-directory>/.hydra/heads.json` |

Every path must be a regular file rather than a symlink. The reader accepts
only project configuration schema version 2 and local metadata schema version
1. Version-1 project configuration is intentionally rejected rather than
migrated because it was never released.

The structured `headsDirectory` reader supports:

- `sibling`, with any non-empty single filename-fragment suffix;
- `relative`, anchored to the stable local project parent and containing only
  normal portable path components;
- `local`, whose absolute path exists only in the non-versioned locator.

Unknown variants, variant-inappropriate fields, unknown storage modes,
separators or control characters in a suffix, and malformed or unsupported
metadata are rejected before mutation.

Hydra validates the installation in this order:

1. read the canonical `projectRoot` and `headsDirectory` from the Git-common
   locator;
2. require its `projectId` to match the versioned configuration;
3. resolve the versioned policy against the locator's stable `projectRoot`,
   never against the current Head;
4. require the resolved path to equal the canonical located directory;
5. require `projectId` and `installationId` to match the ownership marker;
6. reject symlinked directories, repository-internal destinations, and a
   destination nested inside any path reported by `git worktree list`.

Only one Hydra state mutation may proceed at a time. The operation exclusively
creates:

```text
<heads-directory>/.hydra/heads.json.lock
```

The state is read and parsed while that lock is held. Every normal success,
validation failure, cancellation, and pre-commit operational failure releases
the lock. A pre-existing lock is not stolen or removed.

A confirmed unsafe-symlink exclusion is persisted while this same lock is
held. Hydra compares `.hydra.json` byte-for-byte with the version loaded for
the operation, writes and synchronizes a unique sibling temporary file,
atomically renames it over the configuration, and synchronizes the parent
directory on Unix. A concurrent configuration edit is rejected rather than
overwritten. The configuration update is a separately authorized durable
result: if a later full-copy prompt is declined or Head creation fails for an
unrelated reason, the accepted exclusions remain versioned working-tree
changes.

Tracked materialization and overlay planning also use the canonical parent
project as their source. A dirty or divergent calling Head therefore cannot
leak its tracked files, ignored overlays, or configuration changes into a new
sibling Head unless the user names an explicit committed ref available to the
parent repository.

---

## Creation Transaction

After all predictable validation and overlay planning, the implementation
performs:

```text
persist confirmed unsafe-symlink exclusions, if any
        ↓
replan overlays
        ↓
create private branch at baseCommit
        ↓
register worktree without checkout
        ↓
initialize its index from baseCommit
        ↓
materialize tracked entries
        ↓
materialize confirmed overlays
        ↓
verify a clean Git worktree
        ↓
atomically replace heads.json
        ↓
release state lock
```

Branch creation and worktree registration are separate Git operations. A
successful branch creation establishes exact rollback ownership before Hydra
may delete that branch. This prevents an `add` failure caused by a concurrently
created branch from authorizing deletion of a branch Hydra does not own.

The worktree is registered with `--no-checkout`; Hydra writes the index and
working files itself. This avoids an intermediate full checkout before
copy-on-write materialization.

---

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
2. attempts a native CoW clone from the temporary blob;
3. falls back to an exclusive full copy when cloning is unavailable;
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

Overlay files always come from the working tree in which the command runs,
while tracked files come from `baseCommit`.

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
9. probes every regular-file source against the Heads volume and records the
   exact files that need full-copy fallback.

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

---

## State Publication and Rollback

Successful creation adds version-1 metadata containing:

- absolute `worktreePath`;
- full private `headRef`;
- `baseRef` and exact `baseCommit`;
- full local `targetRef`;
- aggregate `materializationBackend` (`cow` or `copy`);
- UTC RFC 3339 `createdAt`.

State publication:

1. re-reads `heads.json` and compares it byte-for-byte with the state loaded
   under the lock;
2. writes and synchronizes a unique temporary JSON file;
3. renames it over `heads.json`;
4. synchronizes the parent directory on Unix;
5. removes the state lock.

A failure before state publication rolls back the registered worktree and the
private branch, then releases the lock. Rollback operates on exact paths and
refs created by the invocation. Cleanup failures retain the original error and
the cleanup diagnostics.

After the state rename, the Head is committed. A subsequent directory-sync or
lock-removal failure is reported as a post-commit cleanup failure, but Hydra
preserves the worktree, branch, and metadata. Rolling them back at that point
would make the already-published state inconsistent.

---

## Verification Contract

CLI integration tests use unique temporary directories and real disposable Git
repositories. Current coverage proves:

- the documented nested command and option syntax;
- default and explicit base/target resolution;
- tracked content comes from `baseCommit`, not uncommitted source edits;
- direct CoW reuse of clean tracked working files without per-file Git blob
  processes, plus blob fallback when tracked content differs;
- persistent tracked-blob protocol handling for multiple blobs, binary
  payloads, SHA-1/SHA-256 headers, invalid requests, destination failures,
  tracked executables, and Unix symlinks;
- independent worktree, index, private branch, and writable files;
- metadata fields and clean Git status;
- Gitignore overlay expansion, negation, conditional fallback confirmation,
  copy, and isolation;
- rejection of unsafe names, unknown refs, missing targets, duplicates,
  existing branches, and existing destinations;
- preservation and isolation of safe relative overlay symlinks;
- explicit, atomic exclusion of multiple absolute or escaping overlay symlinks
  after confirmation, plus cancellation that preserves configuration and Git
  state;
- rejection of tracked-file overlay collisions and unsafe overlay paths not
  covered by the symlink-exclusion repair;
- rejection of malformed, obsolete, newer, or unsupported configuration and
  local metadata;
- stable resolution when creation is invoked from an existing Head;
- `sibling`, `relative`, and `local` directory policies, including Unicode and
  whitespace in safe suffixes;
- rejection of locator/marker ownership mismatches, path separators in a
  suffix, symlinked metadata directories, and directories nested inside another
  registered worktree;
- lock release on pre-commit failures;
- preservation of committed artifacts when only post-commit lock cleanup
  fails;
- bounded parallel overlay hashing with ordered results, argument-limit
  bisection, lossless Unix path-byte handling, isolated exact per-file CoW
  probes, destination identity verification, permission preservation, and
  deduplicated parent paths;
- clean non-interactive `stderr` and phase descriptions for interactive
  progress.

Tests that mutate Git or the filesystem use only their disposable fixture.

An ignored release-mode performance fixture creates a disposable repository
with a configurable large overlay set (10,000 files by default) and reports
complete Head-creation time without asserting a machine-dependent threshold.
It supplies explicit confirmation when the test volume requires full-copy
fallback, so non-CoW environments measure that supported path instead of
failing on non-interactive input:

```bash
cargo test --release -p hydra-cli \
  --test head_create_performance -- --ignored --nocapture
```

---

## Known Implementation Gaps

1. **Cross-Head content-source reuse.** Tracked files can reuse the invoking
   working tree when its tracked state matches `baseCommit`, while overlays
   clone from that same workspace. Hydra does not yet search existing Heads or
   a persistent content cache for another matching source.
2. **Forced fallback coverage in Head creation.** Initialization directly
   verifies both CoW and full-copy behavior. Head-creation tests accept either
   detected backend but do not yet force the per-file fallback path.
3. **Crash reconciliation.** Atomic state publication and rollback protect
   ordinary errors, but process termination between Git/filesystem steps is not
   yet reconciled automatically. A stale state lock is reported and preserved
   for later repair rather than removed heuristically.
4. **Cross-platform symlinks and durability.** Tracked and overlay symlink
   materialization is implemented only on Unix. Direct runtime evidence for
   this workflow currently comes from the development platform and does not
   establish macOS-and-Linux completion by itself.
5. **Submodule population.** Gitlink entries receive an empty directory only;
   submodule initialization and network access are intentionally not implicit.

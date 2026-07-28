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

If `--from` is absent, Hydra resolves `HEAD`. The core returns the created Head
path and effective aggregate storage backend. The CLI prints them only after
the worktree and local state have been committed:

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

The backend line is `Storage backend: full copy` when any materialized regular
file required the safe copy fallback.

For every non-empty overlay plan, successful creation also prints its logical
size and file count:

```text
Overlay: <count> file(s), <bytes> byte(s)
```

This summary is informational and never causes a prompt by itself. During
planning, the core attempts a temporary reflink from each real overlay source
into the Heads directory. It removes the exact probe immediately. When every
probe succeeds, creation proceeds without terminal input.

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

The base is resolved to both:

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

Head creation reads:

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

---

## Creation Transaction

After all predictable validation and overlay planning, the implementation
performs:

```text
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

Tracked entries come from:

```text
git ls-tree -r -z --full-tree <baseCommit>
```

They do not come from the source working tree. Uncommitted edits in the source
therefore remain untouched and do not change the tracked starting content of a
new Head.

Regular blobs are streamed from Git into uniquely named temporary files in the
Heads directory. For each destination Hydra:

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
4. walks only existing entries below the repository root;
5. calculates each selected file's logical size and Git object hash;
6. sorts selected relative paths for deterministic materialization;
7. probes whether each source can be reflinked to the Heads volume and records
   the files that need full-copy fallback.

An absent expanded rules file contributes no rules. An existing expanded file
must be a regular file at a safe relative path.

Overlay protection rejects:

- `.git` and everything below it;
- selected symlinks and special files;
- a selected path that would overwrite a tracked entry;
- an included rules path that is absolute or contains non-normal components;
- a source that no longer resolves inside the repository at materialization
  time.

Each source is revalidated immediately before materialization. Hydra uses the
same CoW-first, exclusive-copy fallback as tracked regular files, but performs
the fallback only after explicit confirmation. It preserves permissions, then
hashes both source and destination. A concurrent content change aborts instead
of publishing a partial Head.

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
- independent worktree, index, private branch, and writable files;
- metadata fields and clean Git status;
- Gitignore overlay expansion, negation, conditional fallback confirmation,
  copy, and isolation;
- rejection of unsafe names, unknown refs, missing targets, duplicates,
  existing branches, and existing destinations;
- rejection of tracked-file overlay collisions and selected symlinks;
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
  fails.

Tests that mutate Git or the filesystem use only their disposable fixture.

---

## Known Implementation Gaps

1. **Content-source reuse.** Tracked files currently clone from temporary Git
   blob files and overlays clone from the current workspace. Hydra does not yet
   search existing Heads for another source with the same content identity.
2. **Forced fallback coverage in Head creation.** Initialization directly
   verifies both CoW and full-copy behavior. Head-creation tests accept either
   detected backend but do not yet force the per-file fallback path.
3. **Crash reconciliation.** Atomic state publication and rollback protect
   ordinary errors, but process termination between Git/filesystem steps is not
   yet reconciled automatically. A stale state lock is reported and preserved
   for later repair rather than removed heuristically.
4. **Cross-platform tracked symlinks and durability.** Tracked symlink
   materialization is implemented only on Unix. Direct runtime evidence for
   this workflow currently comes from the development platform and does not
   establish macOS-and-Linux completion by itself.
5. **Submodule population.** Gitlink entries receive an empty directory only;
   submodule initialization and network access are intentionally not implicit.

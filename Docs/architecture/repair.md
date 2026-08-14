# Hydra Repair

## Purpose

This document defines the implemented reconciliation workflow:

```text
hydra repair
```

The command compares Hydra's local inventory with Git's registered worktrees,
their symbolic branches, the managed filesystem paths, and private branch
refs. Product intent remains authoritative in
[`../product/hydra-mvp-context.md`](../product/hydra-mvp-context.md).

---

## Planning Boundary

The first phase is read-only. It loads and validates the installation and,
when present, the inventory, then parses one:

```text
git worktree list --porcelain -z
```

snapshot containing both paths and symbolic branch refs. Planning does not
acquire `heads.json.lock`, write inventory, move worktrees, delete refs, or
hide inconsistencies. When a recognized lock marker exists, it performs a
non-blocking advisory-lock probe against the stable ownership marker and
immediately releases a successful probe. This changes no persisted state.

Inventory paths retain the same trust boundary as inspection and removal:
every name is validated and every recorded path must equal
`<owned-heads-directory>/<name>`. Unsafe metadata is an error rather than a
repair candidate.

---

## Guided Repairs

Hydra offers only six deterministic repairs, and applies none without an
explicit affirmative answer.

### Abandoned current-version state lock

Every new state transaction writes a versioned `heads.json.lock` marker while
holding an exclusive operating-system advisory lock on the validated
`directory.json` ownership marker. If the process terminates, the OS releases
the advisory lock even though the marker can remain.

Hydra proposes removal only when the marker is a regular file with the exact
current schema and the advisory guard can be acquired without blocking. After
confirmation, the core acquires the guard again, re-reads the marker, and
removes it only if it is still the same supported lock class. The command then
returns so the user can rerun repair against state that is no longer blocked.

An active current lock is report-only. Empty or malformed JSON, unknown fields,
and unsupported versions are invalid local metadata: planning fails and leaves
the file untouched. Repair never uses PID inference, migrates another lock
format, or edits the ownership marker.

### Missing inventory

Every newly created Head stores a versioned recovery manifest in its private
Git linked-worktree administrative directory. The manifest preserves the exact
inventory metadata that Git alone cannot reconstruct: base and target intent,
the resolved base commit, materialization backend, creation time, managed path,
and private ref.

When `heads.json` is absent, Hydra offers to rebuild it only when every
registered worktree using the configured Hydra branch prefix has a manifest
whose Head name, managed path, and symbolic branch agree with current Git and
the owned Heads directory. One missing or inconsistent manifest makes the
complete reconstruction report-only; Hydra does not publish a partial
inventory or infer the missing fields.

A malformed, non-regular, or unsupported recovery manifest is a validation
error rather than a repair candidate. Hydra leaves the missing inventory,
worktree, branch, and manifest untouched for diagnosis.

After confirmation, Hydra acquires the normal state lock, verifies that the
inventory is still absent, rebuilds the complete recovery plan, requires its
Head set to match the approved set exactly, and creates `heads.json`
atomically without replacing an existing file. A malformed or unsupported
inventory is an error and remains byte-for-byte unchanged.

### Manifest-backed untracked Head

When `heads.json` exists but omits a registered Hydra-prefixed worktree, Hydra
can adopt that Head only when:

1. the inventory does not already contain the derived Head name;
2. the registered path is a present directory and its private branch ref still
   exists;
3. the worktree has a current, valid private recovery manifest;
4. manifest name and private ref equal the name and symbolic branch observed
   from Git;
5. the manifest path equals `<owned-heads-directory>/<name>` and the registered
   worktree path exactly.

Hydra reports each qualifying worktree as a recoverable untracked Head. After
confirmation, it acquires the normal state lock, rebuilds the complete
candidate set, and requires that set to equal the approved names exactly. It
then atomically adds the recovered metadata while preserving all existing
inventory entries, worktrees, and branches. The command returns after adoption
so any other inconsistency can be planned again from the new state.

A missing or semantically inconsistent manifest leaves the worktree
report-only. A malformed, non-regular, or unsupported manifest is a validation
error and remains untouched.

### Interrupted pre-worktree creation

Head creation publishes a versioned pending-intent record inside the owned
Heads metadata directory before it creates the private branch. Repair validates
that the record filename, Head name, managed path, configured branch prefix,
base commit, and private ref agree.

The residue is automatically repairable only when no registered worktree and
no filesystem entry exists for the managed path. An absent private ref permits
removal of the journal alone. A present private ref permits cleanup only when
it still points to the exact recorded base commit; deletion uses Git
compare-and-swap so a concurrent advance cannot be discarded. A pending record
left after successful inventory publication is cleaned without changing the
recorded Head or its branch.

After confirmation, Hydra reacquires the normal state lock and rebuilds the
complete candidate set. It deletes an unchanged private ref before removing
the journal, so a journal-cleanup failure retains enough evidence for another
repair. A present or relocated worktree, a present path, an advanced branch,
or inconsistent journal data is preserved for diagnosis.

### Stale inventory

An entry is stale only when all of these facts hold:

1. its managed path is absent;
2. Git has no registered worktree for its private branch;
3. the private branch ref still exists.

After confirmation, Hydra atomically removes only the inventory entry. The
private branch is always preserved, so committed work remains recoverable
through Git.

### Relocated worktree

A relocation is repairable only when:

1. the recorded managed path is absent;
2. exactly one Git worktree is attached to the recorded private branch;
3. that worktree is registered at another path.

After confirmation, Hydra uses `git worktree move` to restore the worktree to
its managed path. It does not relax the owned-directory policy or rewrite the
inventory toward an arbitrary external destination.

---

## Report-only Inconsistencies

Hydra reports but does not guess a mutation for:

- a Hydra-prefixed Git worktree missing from inventory and lacking a matching
  recovery manifest;
- a pending creation that already has a worktree, managed filesystem entry, or
  advanced private branch;
- a registered worktree whose physical directory is missing;
- a present managed directory not registered with Git;
- non-directory entries at managed Head paths;
- missing private branch refs;
- worktree branches that differ from inventory;
- metadata refs that differ from the configured branch prefix;
- ambiguous duplicate branch-to-worktree associations.

An untracked worktree without a matching recovery manifest does not contain
enough authoritative information to reconstruct `baseRef`, `baseCommit`,
`targetRef`, creation time, and the materialization backend. Repair therefore
preserves the worktree and branch and requests manual diagnosis instead of
fabricating intent.

The current command does not remove active lock files, repair locator or
ownership identity, or relocate the whole Heads directory. Missing inventories
containing Heads without recovery manifests remain explicitly report-only.

---

## Mutation and Race Safety

After confirmation, the core acquires the normal exclusive state lock and
rebuilds the repair plan from current Git and inventory state. An approved
entry that is no longer a repair candidate is skipped. Missing-inventory
recovery is stricter: any change to the complete approved Head set cancels the
publication.

Manifest-backed adoption uses the same exact-set rule after acquiring the
normal state lock. A removed or changed manifest, a changed Git association,
or a newly colliding inventory name cancels the complete adoption rather than
publishing a partial approved set.

Abandoned-lock recovery similarly reacquires the stable OS guard after
confirmation. If another process owns it, or if the marker disappeared or
changed class, Hydra preserves the current path and reports that no repair was
applied or that another state operation owns it.

Relocated worktrees are restored before inventory publication. Stale entries
are removed together through one atomic state replacement. Only confirmed
cleanup of an exact pending pre-worktree creation may delete a private branch,
and only with compare-and-swap at its recorded base commit. No repair path
recursively removes a directory, edits tracked project code, or invokes a
shell.

Declining every prompt leaves Git, filesystem, inventory, and refs unchanged.

---

## Verification Contract

Disposable-repository tests prove:

- a consistent project remains byte-for-byte unchanged;
- current-version abandoned lock removal requires confirmation and changes no
  inventory, worktree, or branch state;
- an active current-version lock is detected through the OS guard and
  preserved, including when ownership changes after planning;
- malformed and unsupported lock markers fail validation and remain
  byte-for-byte unchanged;
- missing inventory requires confirmation and remains absent after refusal;
- confirmed recovery reproduces the original inventory bytes from exact
  recovery manifests while preserving worktrees and branches;
- one missing recovery manifest disables complete automatic reconstruction;
- malformed inventory is rejected and preserved rather than replaced;
- a manifest-backed untracked Head requires confirmation and restores its
  exact original metadata without changing existing inventory entries;
- missing or inconsistent manifests do not authorize adoption, and a manifest
  change after planning cancels publication without leaving a lock;
- interrupted pre-worktree creation cleanup requires confirmation, preserves a
  branch changed after planning, and leaves a durable journal when safe cleanup
  cannot be proven;
- stale inventory requires confirmation;
- confirmed stale cleanup removes only metadata and preserves the branch;
- relocated worktrees require confirmation and return to the managed path only
  after approval;
- an untracked Hydra worktree is reported without inferred metadata;
- a registered worktree with a deleted directory is detected without mutation;
- repair paths that start without a lock leave none behind, while refusal and
  report-only classification preserve the pre-existing lock byte-for-byte;
- NUL-delimited Git worktree records keep paths and branches associated,
  including detached worktrees.

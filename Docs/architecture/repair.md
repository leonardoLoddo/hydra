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
hide inconsistencies.

Inventory paths retain the same trust boundary as inspection and removal:
every name is validated and every recorded path must equal
`<owned-heads-directory>/<name>`. Unsafe metadata is an error rather than a
repair candidate.

---

## Guided Repairs

Hydra offers only three deterministic repairs, and applies none without an
explicit affirmative answer.

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

- a Hydra-prefixed Git worktree missing from inventory;
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

The current command does not remove stale lock files, repair locator or
ownership identity, or relocate the whole Heads directory. Missing inventories
from legacy Heads without recovery manifests remain explicitly report-only.

---

## Mutation and Race Safety

After confirmation, the core acquires the normal exclusive state lock and
rebuilds the repair plan from current Git and inventory state. An approved
entry that is no longer a repair candidate is skipped. Missing-inventory
recovery is stricter: any change to the complete approved Head set cancels the
publication.

Relocated worktrees are restored before inventory publication. Stale entries
are removed together through one atomic state replacement. No repair path
deletes a private branch, recursively removes a directory, edits tracked
project code, or invokes a shell.

Declining every prompt leaves Git, filesystem, inventory, and refs unchanged.

---

## Verification Contract

Disposable-repository tests prove:

- a consistent project remains byte-for-byte unchanged;
- missing inventory requires confirmation and remains absent after refusal;
- confirmed recovery reproduces the original inventory bytes from exact
  recovery manifests while preserving worktrees and branches;
- one missing recovery manifest disables complete automatic reconstruction;
- malformed inventory is rejected and preserved rather than replaced;
- stale inventory requires confirmation;
- confirmed stale cleanup removes only metadata and preserves the branch;
- relocated worktrees require confirmation and return to the managed path only
  after approval;
- an untracked Hydra worktree is reported without inferred metadata;
- a registered worktree with a deleted directory is detected without mutation;
- the state lock is absent after success, refusal, and report-only outcomes;
- NUL-delimited Git worktree records keep paths and branches associated,
  including detached worktrees.

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

The first phase is read-only. It loads and validates the installation and
inventory, then parses one:

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

Hydra offers only two deterministic repairs, and applies neither without an
explicit affirmative answer.

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

An untracked worktree does not contain enough authoritative information to
reconstruct `baseRef`, `baseCommit`, `targetRef`, creation time, and the
materialization backend. Repair therefore preserves the worktree and branch
and requests manual diagnosis instead of fabricating intent.

The current command does not remove stale lock files, repair locator or
ownership identity, relocate the whole Heads directory, or reconstruct a lost
inventory. Those cases fail validation or remain explicitly report-only.

---

## Mutation and Race Safety

After confirmation, the core acquires the normal exclusive state lock and
rebuilds the repair plan from current Git and inventory state. An approved
entry that is no longer a repair candidate is skipped.

Relocated worktrees are restored before inventory publication. Stale entries
are removed together through one atomic state replacement. No repair path
deletes a private branch, recursively removes a directory, edits tracked
project code, or invokes a shell.

Declining every prompt leaves Git, filesystem, inventory, and refs unchanged.

---

## Verification Contract

Disposable-repository tests prove:

- a consistent project remains byte-for-byte unchanged;
- stale inventory requires confirmation;
- confirmed stale cleanup removes only metadata and preserves the branch;
- relocated worktrees require confirmation and return to the managed path only
  after approval;
- an untracked Hydra worktree is reported without inferred metadata;
- a registered worktree with a deleted directory is detected without mutation;
- the state lock is absent after success, refusal, and report-only outcomes;
- NUL-delimited Git worktree records keep paths and branches associated,
  including detached worktrees.

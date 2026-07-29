# Head Close

## Purpose

This document defines the implemented close workflow:

```text
hydra head close <name>
```

Closing integrates a clean private Head branch into its recorded target and
then delegates to the protected removal contract in
[`head-removal.md`](head-removal.md).

---

## Safety Preconditions

Hydra requires a fully consistent recorded Head, readable clean worktree,
existing private and target refs, and matching worktree branch and commit. A
dirty, staged, or untracked file blocks close; there is no force option.

The target ref must not be checked out in any registered worktree. Moving a
checked-out ref without updating its files and index would make that worktree
appear dirty, while updating it in place would violate isolation. The current
implementation therefore refuses and asks the user to switch that worktree to
another branch first.

---

## Checkout-Free Integration

Hydra snapshots target and Head commits, then selects:

- no ref update when the Head is already reachable from the target;
- compare-and-swap fast-forward when the target is an ancestor of the Head;
- checkout-free three-way merge when both refs diverged.

Diverged integration uses:

```text
git merge-tree --write-tree <target-commit> <head-commit>
git commit-tree <tree> -p <target-commit> -p <head-commit> -m <message>
git update-ref <target-ref> <new-commit> <expected-target-commit>
```

`commit-tree` uses the repository's normal Git identity configuration. A
missing author identity is an actionable Git failure and leaves both refs
unchanged. `update-ref` publishes only if the target still points to the
validated snapshot, preventing a concurrent target advance from being lost.

A merge conflict produces no commit and no ref mutation. The target, private
branch, worktree, and inventory remain available for explicit resolution.

---

## Removal Composition

After successful integration, Hydra calls the same ordinary protected removal
used by `hydra head remove`. Reachability is rechecked there before deleting
the private branch.

If target publication succeeds but removal fails, Hydra reports the exact
target ref and integrated commit separately. It does not roll back a valid
published integration or force removal; the Head remains available for
inspection and a later retry.

Success prints:

```text
Closed Head payment into refs/heads/main at <commit>
```

---

## Verification Contract

Disposable integration tests prove:

- a Head ahead of its target fast-forwards the target and is removed;
- diverged non-conflicting histories create a merge commit with target and
  Head parents in that order;
- conflicts preserve both refs and the physical Head;
- a target checked out in another worktree is rejected without ref mutation;
- successful close uses protected removal for worktree, inventory, and private
  branch cleanup.

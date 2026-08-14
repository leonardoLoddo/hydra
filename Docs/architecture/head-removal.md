# Head Removal

## Purpose

This document defines the implemented protected removal workflow:

```text
hydra head remove <name> [--force]
```

It owns validation, Git worktree removal, inventory publication, private-branch
cleanup, recoverability, and failure boundaries. Product intent remains
authoritative in
[`../product/hydra-mvp-context.md`](../product/hydra-mvp-context.md).

---

## CLI Contract

Without `--force`, Hydra removes only a Head whose worktree is clean and whose
private branch is fully reachable from its recorded target ref. Success prints:

```text
Removed Head payment
```

`--force` explicitly authorizes discarding tracked, staged, and untracked
worktree changes. It does not authorize unsafe metadata, an unregistered or
missing worktree, a branch mismatch, a detached worktree, or a missing target.

If the private branch contains commits not integrated into the target, forced
removal deletes the worktree and inventory entry but preserves the branch:

```text
Removed Head payment
Preserved branch refs/heads/hydra/payment with unintegrated commits
```

This makes committed work recoverable through normal Git even though
uncommitted files were explicitly discarded.

---

## Validation Boundary

Removal holds the same exclusive `heads.json.lock` used by Head creation.
Before destructive mutation, the core verifies:

1. the Head name uses the supported safe grammar;
2. the inventory contains the name;
3. `worktreePath` is exactly `<owned-heads-directory>/<name>`;
4. the path is a real directory rather than a symlink;
5. Git reports that exact path as a registered worktree;
6. the worktree is attached to the exact recorded private branch;
7. the private branch and target ref both exist;
8. the worktree commit equals the private branch tip;
9. worktree changes and integration status can be read successfully.

The recorded private ref must also equal the configured branch prefix plus the
Head name. `--force` does not bypass any of these trust-boundary checks.

Integration is proven with:

```text
git merge-base --is-ancestor <head-commit> <target-ref>
```

Hydra does not infer integration from branch names, timestamps, or an
ahead/behind count.

---

## Removal Sequence

After validation:

```text
remove registered worktree
        ↓
atomically remove inventory entry
        ↓
remove central recovery record
        ↓
recheck branch reachability from current target
        ↓
delete integrated private ref with expected-object compare
        or preserve unintegrated private ref
        ↓
release state lock
```

Normal removal delegates to `git worktree remove`; forced removal passes
Git's explicit `--force`. Hydra never removes the directory recursively on its
own.

Inventory publication uses the existing byte-for-byte concurrent-state check,
unique temporary file, atomic rename, and directory synchronization. The
central recovery record is removed only after inventory publication and while
the state lock is still held. Its absence is accepted for Heads created before
central records were introduced. The
private ref is deleted only afterward with:

```text
git update-ref -d <head-ref> <expected-head-commit>
```

The expected object prevents a concurrently advanced branch from being
deleted. Target reachability is checked again immediately before deletion; if
the target no longer contains the Head commit, the private branch is
preserved.

---

## Failure and Recovery

Every validation failure leaves worktree, branch, inventory, and lock state
unchanged.

Once Git has removed the worktree, arbitrary uncommitted content cannot be
reconstructed safely. If inventory publication or later branch cleanup fails,
Hydra reports that removal is incomplete and explicitly names the preserved
branch. It never deletes that branch as cleanup for an uncertain state.

The inventory may remain stale if publication fails after worktree removal.
Git and the preserved private branch remain sufficient for diagnosis and
future `hydra repair`; Hydra does not guess at destructive reconciliation.

---

## Verification Contract

Disposable-repository CLI tests prove:

- clean integrated removal deletes worktree, inventory entry, central recovery
  record, and branch;
- a legacy Head without a central recovery record remains removable;
- tracked and untracked changes block ordinary removal without mutation;
- unintegrated commits block ordinary removal without mutation;
- forced removal discards worktree changes while preserving an unintegrated
  branch at its exact commit;
- `--force` cannot redirect deletion through an unsafe recorded path;
- every exercised success and rejection releases the state lock;
- command help documents the safe default and explicit force behavior.

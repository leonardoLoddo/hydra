# Head Close

## Purpose

This document defines the implemented close workflow:

```text
hydra head close <name>
```

Closing either integrates a clean private Head branch into its recorded target
or executes the configured close-command adapter. Optional removal always
delegates to the protected contract in [`head-removal.md`](head-removal.md).

---

## Safety Preconditions

Hydra requires a fully consistent recorded Head, readable clean worktree,
existing private and target refs, and matching worktree branch and commit. A
dirty, staged, or untracked file blocks close; there is no force option.

For native integration, Hydra locates the worktree, if any, that has the target
ref checked out. A checked-out target must still point to the validated target
commit, have no Git operation in progress, and contain no staged, modified,
deleted, or untracked files. A dirty or busy target blocks close before any
integration or removal mutation and reports the observed condition.

---

## Dynamic Native Integration

Hydra snapshots target and Head commits, then selects the integration location:

- when the target ref is checked out in a clean registered worktree, Hydra
  advances that worktree so its ref, index, and files remain synchronized;
- when the target ref is not checked out, Hydra publishes the integration
  checkout-free with compare-and-swap ref updates.

In either location Hydra selects:

- no ref update when the Head is already reachable from the target;
- compare-and-swap fast-forward when the target is an ancestor of the Head;
- a prepared three-way merge commit when both refs diverged.

Checkout-free diverged integration uses:

```text
git merge-tree --write-tree <target-commit> <head-commit>
git commit-tree <tree> -p <target-commit> -p <head-commit> -m <message>
git update-ref <target-ref> <new-commit> <expected-target-commit>
```

For a checked-out target, Hydra still computes a divergent merge with
`merge-tree` and `commit-tree`, then fast-forwards the validated target
worktree to that exact prepared commit. It revalidates the branch, commit,
operation state, and cleanliness immediately before publication and verifies
the resulting ref, index, files, and cleanliness afterward.

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

Native success prints the integration location and result as well as the
target commit. For example:

```text
Closed Head payment into refs/heads/main at <commit>
Integration strategy: target worktree /workspace/Shop
Integration result: fast-forward
```

The strategy is `checkout-free` when no worktree has the target checked out;
the result is `already integrated`, `fast-forward`, or `merge commit`.

Removal commands run from the closing Head use another stable registered
worktree as their control directory. This permits a Head to close itself after
successful native integration, and permits configured close adapters with
`removeOnSuccess` to remove their own Head, without nesting projects or using a
directory that has just been deleted.

---

## Configured Command Adapter

The optional schema-v2 configuration replaces native integration for every
`head close` invocation in that project:

```json
{
  "commands": {
    "close": {
      "strategy": "command",
      "program": "./tools/close-head",
      "args": ["{path}", "{headRef}", "{targetRef}"],
      "removeOnSuccess": true
    }
  }
}
```

`program` and each argument are expanded and passed separately without a
shell. Supported placeholders are `{name}`, `{path}`, `{headRef}`, `{baseRef}`,
and `{targetRef}`. Invalid braces, unsupported placeholders, empty programs,
and NUL bytes are rejected before process creation. The validated Head path is
the adapter's working directory, and standard input, output, and error are
inherited.

Hydra snapshots the target commit before execution and observes it again after
the process exits. A non-zero or signal-based exit preserves the Head and skips
removal. When the target changed, the error reports both the original and
observed commits; when it no longer resolves, the error reports that fact.
Hydra does not attempt to roll back effects produced by trusted project code.

When `removeOnSuccess` is `false`, success preserves the worktree, private
branch, and inventory. When it is `true`, Hydra calls ordinary protected
removal without force. An adapter that did not integrate the private commits,
left changes, or made the Head inconsistent cannot bypass those checks. If
removal fails, Hydra reports separately that the command completed and leaves
the recoverable Head state untouched wherever the adapter itself did not alter
it.

Success prints one of:

```text
Close command completed for Head payment; Head preserved
Close command completed for Head payment; Head removed
```

---

## Verification Contract

Disposable integration tests prove:

- a Head ahead of its target fast-forwards the target and is removed;
- diverged non-conflicting histories create a merge commit with target and
  Head parents in that order;
- conflicts preserve both refs and the physical Head;
- a clean checked-out target is fast-forwarded or merged with its ref, index,
  and files synchronized;
- a dirty checked-out target or target with a Git operation in progress is
  rejected without integration or removal mutation;
- a close invoked from the closing Head completes removal through a stable
  sibling worktree;
- successful native close uses protected removal for worktree, inventory, and
  private branch cleanup;
- a successful command can preserve the Head or remove an integrated Head;
- protected removal failure after command success reports the two phases and
  preserves the Head;
- command failure after a target update reports the before/after commits and
  does not remove the Head.

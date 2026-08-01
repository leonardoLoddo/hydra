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

For native integration, the target ref must not be checked out in any
registered worktree. Moving a checked-out ref without updating its files and
index would make that worktree appear dirty, while updating it in place would
violate isolation. The native strategy therefore refuses and asks the user to
switch that worktree to another branch first.

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
- a target checked out in another worktree is rejected without ref mutation;
- successful native close uses protected removal for worktree, inventory, and
  private branch cleanup;
- a successful command can preserve the Head or remove an integrated Head;
- protected removal failure after command success reports the two phases and
  preserves the Head;
- command failure after a target update reports the before/after commits and
  does not remove the Head.

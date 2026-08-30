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

Hydra accepts close only from the canonical parent project worktree, including
one of its subdirectories. An invocation from any managed Head fails before
loading or executing a configured adapter and reports the canonical parent
path. Native close additionally requires the parent to have the Head's
recorded target ref checked out.

Hydra also requires a fully consistent recorded Head, readable clean Head
worktree, existing private and target refs, and matching Head branch and
commit. For native close, the parent target must still point to the validated
target commit, have no Git operation in progress, and contain no staged,
modified, deleted, or untracked files. Any applicable failed precondition
blocks close before integration, adapter execution, or removal mutation; there
is no force option.

---

## Native Git Integration

Hydra snapshots target and Head commits, validates the parent worktree, then
runs the equivalent of this command there without a shell:

```text
git merge --no-edit <validated-head-commit>
```

Git inherits the terminal's standard input, output, and error. Users therefore
see Git's ordinary already-up-to-date, fast-forward, merge-commit, hook, and
conflict diagnostics. Hydra does not synthesize a commit or resolve conflicts.

When Git leaves a merge conflict, Hydra reports that it is waiting and remains
in the foreground. The user resolves files and commits with normal Git tooling
in the parent worktree, normally through an IDE or another terminal. Hydra
polls the Git operation without holding a Hydra project lock. After the merge
ends, it accepts only a clean target worktree whose current target commit has
the recorded pre-merge target and Head commits as its two parents, in that
order. It then resumes protected Head removal automatically.

If the user runs `git merge --abort`, Hydra recognizes the restored clean
target snapshot, aborts close, preserves the Head, and exits unsuccessfully.
If the operation changes type, targets a different commit, finishes dirty, or
produces a commit with different parents, Hydra stops with an inconsistency
error and does not remove the Head.

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

The result is `already integrated`, `fast-forward`, or `merge commit`. Native
close always reports the canonical parent as its target worktree strategy.

---

## Configured Command Adapter

The optional schema-v2 configuration replaces native integration for every
parent-worktree `head close` invocation in that project:

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
- native Git output is inherited for normal merges;
- conflicts remain available in the parent worktree, and a valid resolution
  commit resumes protected removal automatically;
- `git merge --abort` aborts close and preserves the Head;
- a dirty parent target or target with a Git operation in progress is
  rejected without integration or removal mutation;
- an invocation from a Head reports the parent path without mutation or adapter
  execution;
- the parent must have the recorded target branch checked out;
- successful native close uses protected removal for worktree, inventory, and
  private branch cleanup;
- a successful command can preserve the Head or remove an integrated Head;
- protected removal failure after command success reports the two phases and
  preserves the Head;
- command failure after a target update reports the before/after commits and
  does not remove the Head.

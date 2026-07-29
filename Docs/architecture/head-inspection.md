# Head Inspection

## Purpose

This document defines the implemented read-only model behind:

```text
hydra status
hydra head list
hydra head status <name>
hydra head path <name>
```

The user-visible scope remains authoritative in
[`../product/hydra-mvp-context.md`](../product/hydra-mvp-context.md). Head
creation and state publication remain owned by
[`head-creation.md`](head-creation.md).

---

## Read-only Boundary

Every inspection begins with Git repository and common-directory discovery,
then reuses the same schema-v2 configuration, local locator, directory marker,
ownership, policy, worktree-boundary, and inventory validation used by Head
creation.

Inspection reads `heads.json` without acquiring `heads.json.lock`. It MUST NOT:

- create, remove, or steal the mutation lock;
- rewrite configuration, locator, marker, or inventory;
- create or remove branches, refs, worktrees, or files;
- perform implicit repair;
- hide a detected inconsistency by updating derived state.

A lock owned by a concurrent mutation does not prevent a read-only snapshot.
The snapshot may represent either side of an atomic inventory replacement, but
never a partially serialized state file.

---

## Trust-boundary Validation

Inventory content is external input even though it is local. Before returning
or inspecting a recorded path, Hydra:

1. validates the logical Head name with the creation name grammar;
2. requires `worktreePath` to be absolute;
3. requires it to equal `<validated-heads-directory>/<name>`;
4. rejects a path outside the owned directory instead of invoking Git there.

Malformed JSON, unsupported versions, ownership mismatches, unsafe policy
resolution, and unsafe recorded paths are command errors. Missing worktrees or
refs are inspectable inconsistencies and are reported without mutation.

---

## Command Models

`hydra head list` returns inventory names in `BTreeMap` order, which is stable
lexicographic order. Empty state produces successful empty output.

`hydra head path <name>` returns only the validated absolute recorded path plus
a newline. An unknown name fails without stdout.

`hydra status` returns:

- the discovered repository root;
- the validated physical Heads directory;
- the number of locally registered Heads;
- one ordered `name` and status summary per Head.

Summary status is:

- `inconsistent` when at least one consistency issue exists;
- `clean` when Git reports no staged, unstaged, deleted, or untracked files;
- `modified` otherwise.

`hydra head status <name>` returns recorded intent and observed state:

- path, private branch, current branch commit;
- base ref and exact creation commit;
- target ref;
- modified, added, deleted, and untracked counts;
- ahead/behind;
- worktree presence;
- accumulated consistency issues.

---

## Git Interpretation

Change counts come from:

```text
git status --porcelain=v1 -z --untracked-files=all
```

Hydra parses NUL-delimited records so whitespace and unusual filename bytes do
not alter record boundaries. Each path contributes to one public category:
untracked, deleted, added, or modified. Rename and copy records consume their
second path without counting it as another change.

Ahead/behind comes from:

```text
git rev-list --left-right --count <baseRef>...<headRef>
```

The displayed order is `ahead/behind`, reversing Git's left/right field order.
The comparison uses the current `baseRef`, so advancement of the source branch
is visible. The separately displayed `baseCommit` preserves the exact creation
point. If the recorded base ref no longer exists, Hydra reports the
inconsistency and falls back to `baseCommit` for the comparison.

Consistency checks currently cover:

- missing or non-directory worktree paths;
- worktrees absent from Git's registered worktree list;
- missing private Head refs;
- missing base refs, with comparison fallback to the exact creation commit;
- missing target refs;
- worktree branches that differ from recorded metadata;
- worktree branch or status queries that cannot be read.

---

## Verification Contract

CLI integration tests use disposable Git repositories and prove:

- complete help syntax for all four commands;
- ordered listing and project summaries;
- clean and modified classification;
- exact detailed metadata and change counts;
- ahead and behind changes when the Head and base ref advance independently;
- path-only output;
- unknown-name rejection;
- missing worktree, base-ref, and target-ref diagnostics without repair;
- rejection of recorded paths outside the owned Heads directory;
- byte-for-byte inventory preservation and absence of mutation locks.

Creation integration tests remain representative consumers of the shared state
loader and Git adapter.

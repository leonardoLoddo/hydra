# Recovery and Troubleshooting

Hydra favors recoverability over automatic cleanup. Git remains authoritative
for worktrees, refs, commits, and working-tree state; Hydra's local records
preserve Head intent that Git cannot reconstruct by itself.

Do not edit local Hydra metadata, delete a lock, remove a Head directory, or
run destructive `git worktree` commands to make an error disappear.

## Start with read-only evidence

Run:

```bash
git status
git worktree list
hydra status
hydra head list
hydra head status <name>
```

Then ask Hydra to plan reconciliation:

```bash
hydra repair
```

Planning is read-only. Hydra compares inventory, filesystem paths, Git
worktrees, symbolic branches, refs, pending intent, recovery records, and lock
state. It mutates only one of the deterministic cases below and only after an
explicit affirmative answer.

Empty input or a negative answer applies no repair. After confirmation, Hydra
acquires the normal protection and rebuilds the plan so stale approval does
not authorize a changed state.

## Deterministic repairs

### Abandoned current-version lock

During state mutation Hydra writes `heads.json.lock` while holding an
operating-system advisory lock on the stable ownership marker. After a process
crash, the marker file can remain even though the OS lock was released.

Hydra proposes removal only when the marker is a regular file with the exact
current schema and the OS guard can be acquired without blocking:

```text
Remove the abandoned Hydra state lock? [y/N]
```

An active lock is report-only. Empty, malformed, or unsupported lock content
is a validation error and remains untouched. Hydra does not infer activity
from a persisted PID.

After removing an abandoned lock, rerun `hydra repair` to plan any remaining
issues.

### Missing inventory

Each current Head has an exact central recovery record and a matching private
Git-worktree manifest. If `heads.json` is absent, Hydra can rebuild it only
when every Hydra-prefixed worktree has authoritative evidence matching its
name, managed path, and symbolic branch.

One valid record is sufficient if the other was lost; when both exist they
must match exactly. Hydra asks, for example:

```text
Rebuild the missing inventory with 1 recovered Head? [y/N]
```

Hydra never publishes a partial reconstruction. A Head without evidence or
with disagreeing records makes the complete set report-only. A malformed,
non-regular, or unsupported record is preserved as a validation error.

### Recovery-backed Head missing from inventory

When inventory exists but omits a registered Head, Hydra can adopt it only
when exact recovery evidence agrees with Git and the owned path:

```text
Add 1 recovered Head to the inventory? [y/N]
```

After confirmation, Hydra revalidates the complete approved candidate set and
atomically adds it without changing existing entries, worktrees, or branches.
Without authoritative evidence, the worktree remains report-only.

### Interrupted creation before worktree registration

Head creation writes durable pending intent before branch and worktree
mutation. Repair can clean a pre-worktree residue only when:

- no registered worktree exists;
- the managed filesystem path is absent;
- the private branch is absent or still points exactly to the recorded base
  commit.

An unchanged private branch is deleted with compare-and-swap before the
journal is removed. An advanced branch, present path, or present worktree is
preserved for diagnosis.

### Stale inventory entry

An entry is automatically repairable as stale only when its managed path is
absent, Git has no registered worktree for its private branch, and that branch
still exists. Confirmed repair removes only the inventory entry and preserves
the branch.

### Relocated worktree

When the managed path is absent and exactly one Git worktree is attached to the
recorded private branch elsewhere, Hydra can propose moving it back to its
owned path with `git worktree move`. It does not rewrite inventory to accept an
arbitrary external location.

## Report-only conditions

Hydra reports but does not invent a mutation for:

- a Hydra-prefixed worktree missing from inventory without matching recovery
  evidence;
- a pending creation with a present path, worktree, or advanced branch;
- a registered worktree whose directory is missing;
- a present managed directory not registered with Git;
- a non-directory entry at a managed Head path;
- a missing private branch;
- a worktree branch that differs from inventory;
- a metadata ref that differs from the configured branch prefix;
- ambiguous branch-to-worktree associations;
- locator, ownership, or whole-directory relocation problems;
- malformed inventory or unsupported local-state formats.

Git alone cannot recover the intended base, exact creation commit, target,
backend, and creation time for these cases. Preserve the worktree, branches,
and evidence and report the exact diagnostics.

## Common errors

### `Hydra is already initialized`

The canonical parent already has `.hydra.json`. Do not delete metadata or run
`init` again. Use `hydra status` and `hydra repair` for inspection. Running
`init` from a managed Head also resolves to the already initialized parent.

### Missing `heads.json`

Do not recreate it manually. Run `hydra repair` and approve reconstruction
only when every expected Head appears in the recoverable set. If inventory is
present but malformed, preserve it; deleting it does not convert ambiguous
state into safe recovery.

### `configuration version 1 is not supported`

The version 1 project format was experimental and is not migrated. Restore or
create a current version 2 `.hydra.json` through a supported initialization
workflow. Do not edit local ownership state to imitate migration.

### `unknown field $schema`

Hydra does not currently publish an editor schema. Remove only the `$schema`
entry from the versioned JSON, review the diff, and commit the correction.

### `--target is required`

The `--from` value did not identify a local branch. Pass the intended existing
local integration branch:

```bash
hydra head create experiment --from <COMMIT> --target main
```

### Error normalizing the target ref

The target is not an existing local branch. Check it and retry:

```bash
git branch --list
```

Hydra creates no partial Head when target validation fails.

### `directory ownership does not match`

The locator and directory marker do not describe the same local installation.
Do not rewrite identifiers or claim the directory manually. Current repair
does not change ownership.

### `cannot safely reconstruct configuration for existing Heads`

Hydra found local Heads or operational evidence but no authoritative shared
policy. Recover `.hydra.json` from version control or backup. Hydra refuses to
guess branch prefix, overlays, storage mode, or configured commands.

### Directory policy mismatch

The versioned policy and local locator resolve different Heads directories.
Do not create the newly configured directory or edit the locator. Restore the
reviewed shared configuration without deleting existing Heads. Hydra does not
relocate the complete Heads directory.

### `heads.json.lock` exists

Do not delete it manually. `hydra repair` distinguishes a current active lock
from an abandoned current-version lock using the OS guard. Malformed or
unsupported markers are preserved for diagnosis.

### `Head removal is incomplete`

Git may already have removed the worktree while Hydra failed to finish
inventory or branch cleanup. Preserve the private branch named by the error
and run `hydra repair`. A deterministic stale-entry repair removes only the
inventory record and leaves the branch recoverable.

### `Unsafe overlay symlinks`

Hydra selected absolute, broken, escaping, or unsupported symlinks. Approve
exclusion only if the listed links are unnecessary in the Head. Hydra updates
`.hydra.json`; review and commit the change. If a link is required, decline
and replace it with a relative link that resolves entirely inside the project.

### Close is blocked by the target

A checked-out target with staged, modified, deleted, or untracked files, or an
active Git operation, blocks native close. Do not stash, reset, switch, or
delete another task's work merely to continue. Finish or preserve that work in
its owning workflow, then retry close.

### Close reports a conflict

Native close preserves target ref, private branch, Head worktree, and inventory
on conflict. Hydra does not start a partially resolved merge for you. Inspect
the two branches and choose an explicit Git resolution strategy while keeping
the Head recoverable.

### Custom close command failed

Hydra preserves the Head and skips removal. If the trusted command changed or
deleted the target ref before failing, Hydra reports the before/after state but
cannot roll back external effects. Inspect the script, Git refs, pushes, pull
requests, and services it may have touched.

## Storage diagnosis

For backend or probe failures, run:

```bash
hydra doctor storage
```

The command tests the actual managed volume and reports cleanup failures with
their exact path. It does not repair general inventory or ownership problems.

## When to stop

Stop and preserve evidence when:

- Hydra classifies a condition as report-only;
- ownership, locator, or policy does not match;
- recovery records disagree or are malformed;
- a branch advanced after a failure;
- an adapter caused unreviewed external effects;
- a destructive action would require manual deletion or metadata editing.

Include `hydra --version`, `hydra status`, the relevant `head status`,
`git worktree list`, and the exact error in a bug report. Remove secrets and
private path information before sharing logs publicly.

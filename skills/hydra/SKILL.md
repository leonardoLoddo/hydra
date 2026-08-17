---
name: hydra
description: Use Hydra's Git-native CLI to create and operate isolated development Heads for parallel AI-agent or human work. Use when an agent must start a task without sharing a working tree or index, move work into an existing Hydra Head, inspect Head state, or safely finish, integrate, repair, or remove a Head without bypassing Hydra's protections.
---

# Hydra

Use Hydra as the workspace boundary for one task. Keep Git as the source of
truth and use only documented Hydra commands for Head lifecycle operations.

## Establish the starting state

1. Run `hydra --version` and `hydra --help`. Never assume an installed build
   supports a command or option that its help does not show.
2. Resolve the current worktree root with `git rev-parse --show-toplevel` and
   inspect `git status --short --branch`.
3. Run `hydra status` before mutation. Its `Project:` line is the canonical
   parent root even when the current worktree is a Head.
4. If Hydra reports that the project is not initialized, run `hydra init
   [PATH]` only when the user has
   authorized Hydra project setup. Treat the generated `.hydra.json` as shared,
   versioned policy and review it before proposing a commit. Init may reuse an
   exact owned Heads directory only when its inventory and operational content
   are empty. If existing Heads require configuration recovery, stop and
   recover the authoritative `.hydra.json` from version control or backup;
   never accept guessed defaults or edit ownership metadata.
5. Run `hydra head list` and decide whether this task should resume an existing
   Head or create a new one. Never reuse a Head merely because its name looks
   related: inspect it with `hydra head status <name>` and stop if its ownership,
   task, branch, path, or existing work is unclear.

Hydra lifecycle commands may run from the main project worktree or any managed
Head. Hydra uses the shared locator to normalize every invocation to the
canonical parent project, even when `.hydra.json` is missing or stale in the
calling Head. Configuration, Git defaults, overlays, project reporting, and
inventory therefore behave exactly as if the command ran from the parent.
Creating a Head from another Head creates a sibling in the project's Heads
directory; it never creates a nested Head or a second Hydra project hierarchy.
Without explicit `--from` or `--target`, use the parent project's `HEAD` and
local branch, never the calling Head's private branch or files.
Running `hydra init` from a managed Head must report the canonical parent as
already initialized, never initialize the Head as a separate project.

If `.hydra.json` is not part of the selected base commit, it will not appear in
the new Head. Hydra lifecycle discovery still works through the shared locator;
do not copy configuration or local metadata into the Head merely to make it
work.

Do not assume uncommitted changes in the current worktree will enter a new
Head. A Head starts from the commit resolved by `--from`. If another task owns
the current changes, leave them untouched and choose a committed base.

## Select or create a Head

When the user identifies an existing Head, or inspection proves that it belongs
to this task, do not recreate it. Resolve it with `hydra head path <name>`,
inspect its status, and continue with the boundary checks below.

For a new Head:

- Choose a short task-specific name, preferably lowercase words separated by
  hyphens.
- Select `--from` deliberately. Use the intended local branch when the Head
  should start from that branch's current commit.
- Select `--target` deliberately. It is the local branch that `head close`
  will eventually update.
- When `--from` is a detached commit, tag, or other non-branch source, always
  provide `--target`.
- Report the chosen name, source, and target when they are not already explicit
  in the user's request.

Create the Head with the syntax supported by the installed CLI:

```bash
hydra head create <name> --from <source> --target <target>
```

Before creation, review the canonical project's `.hydra.json`, when present.
Pay particular attention to overlay policy and configured `open` or `close`
commands. `storage.mode: "copy"` deliberately forces full copies of regular
tracked and overlay files for deterministic tests or automation; report that
cost-relevant policy before creation. Do not copy configuration or local
metadata into a Head.

Creation may pause for confirmation:

- If Hydra lists unsafe overlay symlinks, decline unless the user explicitly
  authorizes excluding the listed paths from shared policy. Ensure no person,
  editor, or other tool is concurrently editing `.hydra.json` before answering:
  Hydra rejects changes visible at its final comparison and publishes a
  complete file atomically, but portable filesystems do not provide a content
  compare-and-swap against an external save in the final pre-rename window.
  After approval, inspect and report the `.hydra.json` diff; commit it only
  when authorized.
- If Hydra reports that full copy is required, report the file count and byte
  size and answer yes only after the user authorizes that storage cost.
- A negative answer or EOF is a safe cancellation. Do not bypass either prompt
  with manual copying, Git worktree commands, or metadata edits.

If creation fails, do not substitute `git worktree add`, manual directory
copying, or metadata edits. Inspect `hydra status`, `git worktree list`, and the
reported error, then stop if ownership or state is ambiguous.

If Hydra reports failure while normalizing the target ref, verify that the
explicit `--target` names an existing local branch. If it reports a versioned
directory-policy mismatch, do not create the configured directory or edit the
locator; restore the reviewed project configuration or stop for user guidance.

## Move all task work into the Head

1. Obtain the authoritative directory with `hydra head path <name>`.
2. Set every task, edit, build, and test command's working directory to that
   exact path. Lifecycle commands may still be launched there because Hydra
   resolves their control context to the parent project.
3. Verify the boundary with:

```bash
git rev-parse --show-toplevel
git branch --show-current
git status --short --branch
```

4. Confirm the branch is the private Head branch reported by
   `hydra head status <name>`.
5. Read the Head's repository instructions and routed documentation before
   changing files.

Keep temporary files, generated files, tests, and edits inside the Head. Do
not modify another worktree to make this task pass.

Open a Head with `hydra head open <name>` only when the user wants the
configured tool launched. Inspect `commands.open` in the canonical project
configuration first and report the program being started. Do not invent or
silently add an opener when none is configured.

## Develop and verify

- Follow the repository's own implementation, testing, safety, documentation,
  and commit rules.
- Inspect `git status`, `git diff`, and `git diff --staged` from the Head.
- Run the focused checks and complete quality gates required by the repository.
- Commit only when the user and repository instructions authorize it. Create
  commits only on the Head's private branch; never commit this task from the
  target branch.
- Use `hydra head status <name>` from any initialized worktree when lifecycle
  state must be checked.

## Hand off or integrate

Default to leaving a completed Head intact for review. Report its name, path,
branch, status, tests, and whether its commits have been integrated.

Before closing, inspect `commands.close` in the canonical project
configuration. If it is absent or uses the native merge strategy, run `hydra
head close <name>` only when the user has authorized integration and the Head
is clean. The command may run from the main project or any initialized Head,
including the Head being closed. If the target branch is checked out in a clean
worktree, Hydra integrates there and keeps its ref, index, and files in sync. If
the target is not checked out, Hydra integrates checkout-free.

If `commands.close` uses a custom command, report its program, arguments, and
`removeOnSuccess` policy before execution. Treat the command as trusted project
code that is not sandboxed and may push branches, create pull requests, modify
files, or contact services. Execute it only when the user's authorization
covers those concrete effects. Do not describe a successful custom command as
an integration unless its observed result proves that claim.

A target worktree with staged, modified, deleted, or untracked files, or with a
Git operation in progress, blocks close without mutating the target or Head.
Report the blocker; do not switch branches, stash another task's files, or
alter another worktree to force integration. Report Hydra's integration
strategy and result after a successful close.

When closing the Head from its own directory, expect that directory to be
removed on success. Change the caller's working directory to the parent project
or another surviving Head before issuing subsequent commands.

Use `hydra head remove <name>` only for an authorized removal after inspecting
the Head. Never use `--force` unless the user explicitly authorizes discarding
the identified uncommitted files. Force removal must still preserve commits
that are not integrated, and its output must be reported.

## Handle inconsistencies safely

- Treat `hydra status`, `hydra head status <name>`, `hydra head list`, and
  `hydra head path <name>` as the read-only inspection surface.
- Never edit `.git/hydra`, the Heads directory's `.hydra` files, locator,
  ownership marker, inventory, or lock by hand.
- Never replace Hydra lifecycle commands with `rm`, recursive deletion, or
  destructive `git worktree` commands.
- Do not delete a lock by hand merely because it appears stale. Run `hydra
  repair` read-only first. Authorize its dedicated lock removal only when Hydra
  classifies a current-version lock as abandoned through the OS guard. Stop and
  preserve active locks; treat malformed or unsupported locks as validation
  errors, not formats to migrate.
- Use `hydra doctor storage` when the active backend or fallback behavior needs
  diagnosis. Report its result and any temporary path that Hydra could not
  clean up; do not remove such a path blindly.
- Run `hydra repair` first to collect its plan and decline each proposed
  mutation. Report the exact deterministic repairs and unresolved
  inconsistencies, then rerun and confirm only the changes the user explicitly
  authorizes. When the inventory is missing, approve reconstruction only if
  every expected Head appears in the recoverable set; Hydra will revalidate
  the complete set and use exact central or private recovery records rather
  than infer metadata from Git. Either record is sufficient; when both exist,
  they must match exactly. A Head without recovery evidence, or with
  disagreeing records, disables partial automatic reconstruction, while a
  malformed or unsupported record is a validation error to preserve for
  diagnosis. When an existing inventory omits a recovery-backed worktree,
  authorize adoption only if the
  proposed Head name, managed path, and private branch all match the expected
  task; Hydra must revalidate the complete approved set under lock and preserve
  existing entries. If repair removes an abandoned lock or adopts a Head, rerun
  it before authorizing any other proposal. Repair does not justify editing
  recovery records or lock markers, rewriting ownership or locator data,
  replacing malformed inventory, or reconstructing ambiguous metadata by hand.
- Authorize pending-creation cleanup only when Hydra reports no associated
  worktree or managed path and an absent or unchanged private branch at the
  recorded base commit. Hydra revalidates under lock and deletes that ref with
  compare-and-swap. Preserve and report any pending creation with a present
  worktree, filesystem entry, or advanced branch; never delete its journal,
  branch, or directory manually.

Preserve recoverability over convenience: leave the Head and its private branch
in place whenever safe integration or removal cannot be proven.

## Manage this skill only when requested

Do not update or remove the active Hydra skill merely because a newer Hydra
binary exists. When the user explicitly asks to manage it, use `hydra skill
status codex` first. Use `hydra skill update codex` or `hydra skill remove
codex` only after the user confirms the destination and action. Hydra preserves
unknown or locally modified skill content; do not bypass that refusal with a
manual copy or recursive deletion.

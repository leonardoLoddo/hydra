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
   versioned policy and review it before proposing a commit.

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

## Select a Head identity

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

If creation fails, do not substitute `git worktree add`, manual directory
copying, or metadata edits. Inspect `hydra status`, `git worktree list`, and the
reported error, then stop if ownership or state is ambiguous.

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

Run `hydra head close <name>` only when the user has authorized integration and
the Head is clean. The command may run from the main project or any initialized
Head, including the Head being closed. If the target branch is checked out in
a clean worktree, Hydra integrates there and keeps its ref, index, and files in
sync. If the target is not checked out, Hydra integrates checkout-free.

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
- Do not delete a lock merely because it appears stale. First report the
  operation, processes, worktrees, branches, and Hydra status that can be
  observed safely.
- Run `hydra repair` only after inspection and when the user authorizes the
  proposed state change. If ownership, paths, policy, or Git state remain
  ambiguous, stop and report the exact evidence.

Preserve recoverability over convenience: leave the Head and its private branch
in place whenever safe integration or removal cannot be proven.

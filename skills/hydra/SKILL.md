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
2. Resolve the repository root with `git rev-parse --show-toplevel` and inspect
   `git status --short --branch`.
3. Check whether `.hydra.json` exists at the repository root.
4. If the project is initialized, run `hydra status` before any mutation.
5. If it is not initialized, run `hydra init [PATH]` only when the user has
   authorized Hydra project setup. Treat the generated `.hydra.json` as shared,
   versioned policy and review it before proposing a commit.

If `.hydra.json` is not part of the selected base commit, it will not appear in
the new Head. In that case, keep running Hydra lifecycle commands from the
initialized source worktree; do not copy the configuration or local metadata
into the Head merely to make discovery work.

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
   exact path. Use the initialized source worktree only as the control directory
   for Hydra lifecycle commands when the Head lacks `.hydra.json`.
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
  source worktree or target branch.
- Use `hydra head status <name>` from the initialized source worktree or another
  worktree that contains the versioned Hydra configuration when lifecycle state
  must be checked.

## Hand off or integrate

Default to leaving a completed Head intact for review. Report its name, path,
branch, status, tests, and whether its commits have been integrated.

Run `hydra head close <name>` only when the user has authorized integration and
the Head is clean. This command may update the target ref and remove the Head
after successful integration. Do not switch branches or alter files in another
worktree merely to make close succeed; if the target is open elsewhere, report
the blocker and preserve the Head.

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

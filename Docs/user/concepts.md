# Core Concepts

Hydra adds an isolated-workspace lifecycle around standard Git primitives. It
does not replace Git, your editor, or your development tools.

## Head

A **Head** is a complete directory backed by a linked Git worktree. Each Head
has:

- its own working tree and index;
- a private local branch, such as `hydra/payment`;
- a recorded base ref and exact base commit;
- a recorded local target branch for later integration;
- tracked files from the base commit;
- selected ignored or untracked files copied as overlays;
- a recorded effective storage backend.

Because the directory contains ordinary files, you can open it in any editor,
terminal, build tool, or AI agent without a Hydra-specific integration.

## Parent project and Heads directory

With the default policy, Hydra places Heads beside the repository:

```text
/workspace/
├── Shop/
│   ├── .git/
│   ├── .hydra.json
│   └── ...
└── Shop.heads/
    ├── auth/
    └── payment/
```

Keeping Heads outside the parent working tree avoids recursive
materialization, accidental tracking, and unnecessary editor or watcher load.

Every managed worktree shares the same Git common directory. Hydra uses a
local locator there to normalize lifecycle commands back to the canonical
parent project. You may therefore run Hydra from the parent or from any
managed Head.

Creating a Head while your shell is inside another Head still creates a
sibling under `Shop.heads/`. It does not create a nested project. Unless you
pass `--from` or `--target`, defaults come from the canonical parent project,
not from the calling Head's private branch or files.

## Base, source, private branch, and target

These values serve different purposes:

| Value | Meaning |
|---|---|
| `baseRef` | The source expression or normalized ref selected at creation |
| `baseCommit` | The exact commit used to populate the Head |
| `headRef` | The private branch owned by the Head |
| `targetRef` | The local branch intended to receive the completed work |

For example:

```bash
hydra head create payment --from beta --target main
```

creates a private branch such as `hydra/payment` at the commit currently
resolved by `beta`, while recording `main` as the integration target.

If the source resolves to a local branch and `--target` is omitted, that branch
becomes the target. A detached commit, tag, or non-local source requires an
explicit local target.

Hydra records both the source intent and the exact commit. A Head does not
move automatically when the source branch advances.

## Isolation and direct Git use

Working tree, index, and uncommitted files are isolated per Head. Git objects
remain shared, so a commit created in a Head is immediately visible from the
parent repository.

Inside a Head, normal Git commands remain available:

```bash
git status
git diff
git add .
git commit
git fetch
git rebase
git merge
git push
```

Direct Git operations can make Hydra metadata inconsistent—for example, by
checking out a different branch in the Head or moving its worktree manually.
Use direct Git for development and history operations, but use Hydra's guarded
commands for the Head lifecycle.

Hydra does not automatically pull, push, rebase, merge in the background, or
resolve conflicts. Native integration happens only when you explicitly run
`hydra head close` from the canonical parent project. Hydra runs normal Git
there; after a conflict it waits for your merge commit or abort.

## Tracked files and overlays

Tracked files always come from `baseCommit`. Uncommitted tracked changes in
the parent or another Head are not included.

Overlays select ignored or untracked files from the canonical parent project,
using Gitignore-compatible rules. The default configuration expands the
project's `.gitignore`:

```json
{
  "overlay": {
    "copy": ["... .gitignore"]
  }
}
```

This can place local files such as `.env` or ignored tool state into a new
Head. Overlay content remains ordinary local content: Hydra does not remove
secrets, make them safe to commit, or synchronize later changes.

See [Storage and overlays](storage-and-overlays.md) before copying dependency
directories or accepting a symlink or full-copy prompt.

## Copy-on-write and full copy

Hydra shares Git's object database and tries to materialize regular files with
the native copy-on-write primitive of the actual Heads volume:

- APFS clone on compatible macOS volumes;
- reflink on compatible Linux volumes;
- ReFS block clone on compatible Windows volumes;
- isolated full copy when native cloning is unavailable.

Copy-on-write shares physical blocks initially, but each path behaves as an
independent file. Hydra never uses mutable hard links as a fallback.

Correct isolation is required; physical space savings are conditional. Run
`hydra doctor storage` to test the real destination volume.

## Shared configuration and local state

Hydra separates versioned project policy from machine-specific state:

| Data | Location | Commit it? |
|---|---|---:|
| Shared policy | `<project>/.hydra.json` | Yes, after review |
| Project locator | `<git-common-dir>/hydra/project.json` | No |
| Directory ownership | `<heads-directory>/.hydra/directory.json` | No |
| Head inventory | `<heads-directory>/.hydra/heads.json` | No |
| Pending/recovery records | Hydra local metadata and private worktree Git data | No |

Never copy local state between machines or edit it manually. It contains
paths, ownership identities, and recovery evidence specific to one local
installation.

## Lifecycle at a glance

```text
Git repository
    │
    ├─ hydra init
    │      └─ shared policy + owned local Heads directory
    │
    ├─ hydra head create
    │      └─ private branch + isolated worktree
    │
    ├─ edit, test, commit, inspect
    │
    ├─ return to parent with target checked out
    │
    ├─ hydra head close
    │      └─ native Git merge + protected removal
    │
    └─ hydra repair
           └─ explicit deterministic reconciliation only
```

`hydra head remove` is a separate cleanup action. Without `--force`, it
requires a clean worktree and commits already integrated into the target.

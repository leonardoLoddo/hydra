# Hydra User Guide

Hydra is a Git-native workspace manager for isolated development **Heads**.
Each Head is a real Git worktree with its own working tree, index, and private
branch. Humans, editors, and AI agents can work on several tasks without
sharing uncommitted files.

This is the entry point for Hydra's maintained English user documentation.
The documentation describes only behavior available in the current binary.

> [!IMPORTANT]
> Hydra is an early preview. Use it on repositories whose important work is
> committed or backed up. When Git, filesystem, ownership, or recovery state
> is unclear, stop and inspect it before attempting any manual repair.

The [latest public preview](https://github.com/leonardoLoddo/hydra/releases/latest)
is available from GitHub Releases. An
[Italian user guide](hydra-user-guide.it.md) is also maintained.

## Start here

New users should read these pages in order:

1. [Installation and updates](installation.md) — supported platforms,
   Homebrew, source installation, upgrades, removal, and shell completion.
2. [Core concepts](concepts.md) — Heads, private branches, source and target
   refs, storage, overlays, and the local/shared state boundary.
3. [Head workflows](head-workflows.md) — initialize a project, create, inspect,
   open, integrate, and remove Heads.

Use the focused guides when you need more control:

- [Configuration](configuration.md) explains `.hydra.json`, directory
  policies, branch naming, and open/close command adapters.
- [Storage and overlays](storage-and-overlays.md) explains copy-on-write,
  full-copy fallback, ignored files, symlinks, prompts, and diagnostics.
- [Agent Skills](agent-skills.md) explains the optional portable skill and its
  safe provider-specific install, update, and removal lifecycle.
- [Recovery and troubleshooting](recovery-and-troubleshooting.md) explains
  `hydra repair`, recoverable states, report-only inconsistencies, and common
  errors.
- [CLI reference](cli-reference.md) is a compact index of every current command
  and option.

## Five-minute workflow

Hydra requires an existing Git repository with at least one commit:

```bash
cd /path/to/project
git status
hydra init
```

Review and commit the generated shared policy:

```bash
git diff -- .hydra.json
git add .hydra.json
git commit -m "chore: configure Hydra"
```

Create a Head from a local branch and record the branch that should later
receive the work:

```bash
hydra head create payment --from main --target main
```

Move into the path reported by Hydra:

```bash
cd "$(hydra head path payment)"
git status
git branch --show-current
```

Develop normally. Git commands, editors, build tools, and agents operate on
ordinary files inside the Head:

```bash
git add .
git commit -m "feat: implement payment flow"
hydra head status payment
```

When the Head is clean and ready, explicitly integrate it into its recorded
target and remove the completed worktree:

```bash
hydra head close payment
```

Read [Head workflows](head-workflows.md) before closing divergent work or
using forced removal. A conflict, dirty target, or active Git operation blocks
native close without deleting the Head.

On WSL 2, installing through Homebrew does not imply that the current volume
supports reflinks. Run `hydra doctor storage`; the default ext4 root commonly
uses full copy. For a supported XFS-backed setup, follow
[WSL copy-on-write storage](storage-and-overlays.md#wsl-2-copy-on-write) before
initializing a fresh clone.

## Safety model

Keep these rules in mind:

- A new Head starts from a committed ref or commit. It does not capture
  uncommitted changes from another worktree.
- Hydra never synchronizes a Head with its source branch in the background.
- Each Head works on a private branch such as `hydra/payment`.
- `hydra head close` is an explicit integration action. It can update the
  recorded target branch and normally removes the Head afterward.
- `hydra head remove --force` may permanently discard tracked, staged, and
  untracked worktree changes. It does not bypass ownership or path checks.
- Hydra preserves an unintegrated private branch during forced removal, so
  committed work remains recoverable through Git.
- Do not edit Hydra's local locator, inventory, ownership marker, recovery
  records, or lock files by hand.
- Do not replace protected Hydra lifecycle commands with recursive deletion or
  destructive `git worktree` commands.
- Treat project-configured `open` and `close` programs as trusted code. Hydra
  separates their arguments but does not sandbox them.

## What Hydra manages

Hydra currently manages:

- initialization of one Git repository;
- isolated Head creation with private branches;
- tracked-file and selected-overlay materialization;
- read-only project and Head inspection;
- configured opening commands;
- native or configured close workflows;
- protected removal;
- deterministic repair proposals;
- real storage diagnostics;
- Bash, Zsh, and Fish completions;
- an optional Agent Skill for Codex, Gemini CLI, and Antigravity CLI.

## Getting help

The installed binary is authoritative for syntax:

```bash
hydra --help
hydra head --help
hydra <command> --help
```

If documentation and the installed help differ, first check which executable
you are running:

```bash
hydra --version
command -v hydra
which -a hydra
```

Report preview defects through the repository's
[issue templates](https://github.com/leonardoLoddo/hydra/issues/new/choose).
Use the private process in
[`SECURITY.md`](../../SECURITY.md) for security-sensitive reports.

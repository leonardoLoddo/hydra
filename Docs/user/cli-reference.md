# CLI Reference

This page summarizes the command surface of the current Hydra binary. The
installed help remains authoritative for exact syntax:

```bash
hydra --help
hydra <command> --help
```

## Command index

```text
hydra init [PATH]
hydra status
hydra repair
hydra doctor storage
hydra completions <SHELL>

hydra skill install <PROVIDER> [--yes | --no]
hydra skill status <PROVIDER>
hydra skill update <PROVIDER> [--yes | --no]
hydra skill remove <PROVIDER> [--yes | --no]

hydra head create <NAME> [--from <REF>] [--target <BRANCH>]
hydra head list
hydra head status <NAME>
hydra head path <NAME>
hydra head open <NAME>
hydra head close <NAME>
hydra head remove <NAME> [--force]
```

## Global options

| Option | Purpose |
|---|---|
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print the executable version |

## Project commands

### `hydra init [PATH]`

Initializes the Git repository containing `PATH`; the default is `.`. Creates
the versioned `.hydra.json` policy and locally owned Heads state after path,
ownership, and storage validation.

### `hydra status`

Prints the canonical parent project, managed Heads directory, count, and one
`clean`, `modified`, or `inconsistent` summary per Head. Read-only.

### `hydra repair`

Plans reconciliation between Hydra state and Git worktrees. Ambiguous states
remain report-only; deterministic mutations require confirmation.

### `hydra doctor storage`

Runs a real copy-on-write and full-copy isolation probe on the managed Heads
volume. Reports the native primitive, execution environment, and filesystem
when Linux exposes it. A WSL full-copy result includes guidance toward a
reflink-capable Linux volume. Requires an initialized, internally consistent
project.

### `hydra completions <SHELL>`

Prints dynamic completion registration. Supported values are `bash`, `zsh`,
and `fish`.

## Head commands

### `hydra head create <NAME>`

Creates an isolated Head.

| Option | Meaning |
|---|---|
| `--from <REF>` | Source ref or commit; defaults to canonical parent `HEAD` |
| `--target <BRANCH>` | Existing local branch intended for integration |

When the source is not a local branch, `--target` is required. Overlay
symlinks and full-copy fallback can cause default-negative prompts.

### `hydra head list`

Prints local Head names in stable order, one per line. Read-only.

### `hydra head status <NAME>`

Prints recorded intent, observed Git/worktree state, changes, ahead/behind,
and consistency diagnostics. Read-only.

### `hydra head path <NAME>`

Prints only the validated absolute path. It is suitable for:

```bash
cd "$(hydra head path <NAME>)"
```

### `hydra head open <NAME>`

Validates the Head and starts `commands.open`. Fails if no opener is
configured. Does not require a clean Head.

### `hydra head close <NAME>`

Requires a clean, consistent Head. Uses native local integration and protected
removal unless `commands.close` selects a trusted command adapter. There is no
force option.

### `hydra head remove <NAME>`

Ordinary removal requires a clean Head and commits already integrated into the
recorded target.

`--force` authorizes discarding uncommitted tracked, staged, and untracked
files. It does not bypass path, ownership, branch, worktree, or target checks,
and it preserves a private branch containing unintegrated commits.

## Skill commands

Supported providers are `codex`, `gemini`, `agy`, and `antigravity`. They
install the same canonical skill into `$HOME/.agents/skills/hydra`,
`$HOME/.gemini/skills/hydra`,
`$HOME/.gemini/antigravity-cli/skills/hydra`, and
`$HOME/.gemini/config/skills/hydra`, respectively.

### `hydra skill install <PROVIDER>`

Installs the packaged skill after a default-negative confirmation. `--yes`
confirms and `--no` declines without interactive input; they are mutually
exclusive.

### `hydra skill status <PROVIDER>`

Reports whether the destination contains a current, unmodified copy managed by
Hydra.

### `hydra skill update <PROVIDER>`

Updates only an unmodified, Hydra-managed copy. Supports mutually exclusive
`--yes` and `--no` automation flags.

### `hydra skill remove <PROVIDER>`

Removes only an unmodified, Hydra-managed copy. Supports mutually exclusive
`--yes` and `--no` automation flags.

## Output and composition notes

- Operational errors use a non-zero exit status and are written to `stderr`.
- Interactive Head creation progress is written to `stderr`; redirected
  execution omits it.
- `head path` preserves the exact path for non-terminal pipelines while human
  output escapes control characters.
- Inspection commands do not repair or rewrite state.
- Dynamic shell completion suppresses project-discovery errors and returns no
  candidates outside a readable Hydra project.

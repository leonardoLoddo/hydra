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
hydra status [--json]
hydra repair
hydra doctor storage [--json]
hydra completions <SHELL>

hydra skill install <PROVIDER> [--yes | --no]
hydra skill status <PROVIDER> [--json]
hydra skill update <PROVIDER> [--yes | --no]
hydra skill remove <PROVIDER> [--yes | --no]

hydra head create <NAME> [--from <REF>] [--target <BRANCH>] [--dry-run [--json]]
hydra head list [--json]
hydra head status <NAME> [--json]
hydra head path <NAME> [--json]
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
ownership, and storage validation. On native Windows, a verified full-copy
fallback links to [Windows copy-on-write setup](windows-copy-on-write.md); WSL
links to [WSL 2 copy-on-write setup](wsl-copy-on-write.md).

### `hydra status`

Prints the canonical parent project, managed Heads directory, count, and one
`clean`, `modified`, or `inconsistent` summary per Head. `--json` returns the
same ordered project summary as a versioned object. Read-only.

### `hydra repair`

Plans reconciliation between Hydra state and Git worktrees. Ambiguous states
remain report-only; deterministic mutations require confirmation.

### `hydra doctor storage`

Runs a real copy-on-write and full-copy isolation probe on the managed Heads
volume. Reports the native primitive, execution environment, and filesystem
when Linux exposes it. Native Windows and WSL full-copy results link to their
respective [Windows](windows-copy-on-write.md) and
[WSL 2](wsl-copy-on-write.md) setup guides. Requires an initialized,
internally consistent project.
Add `--json` for stable machine identifiers and typed capability fields.

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
| `--dry-run` | Validate and print the creation plan without making changes |
| `--json` | Emit the dry-run plan as versioned JSON; requires `--dry-run` |

When the source is not a local branch, `--target` is required. Overlay
symlinks and full-copy fallback can cause default-negative prompts. On native
Windows or WSL, a detected full-copy fallback links to the platform setup guide
before the storage-cost decision, or in the final output when no prompt was
needed.

### `hydra head list`

Prints local Head names in stable order, one per line. `--json` wraps the
ordered names in a versioned object. Read-only.

### `hydra head status <NAME>`

Prints recorded intent, observed Git/worktree state, changes, ahead/behind,
and consistency diagnostics. `--json` separates them into `recorded`,
`observed`, and `consistency` objects. Read-only.

### `hydra head path <NAME>`

Prints only the validated absolute path. It is suitable for:

```bash
cd "$(hydra head path <NAME>)"
```

Add `--json` when the caller also needs the Head name and schema version. The
plain form remains the correct choice for shell command substitution.

### `hydra head open <NAME>`

Validates the Head and starts `commands.open`. Fails if no opener is
configured. Does not require a clean Head.

### `hydra head close <NAME>`

Must be run from the canonical parent project worktree and requires a clean,
consistent Head. The native path also requires the recorded target branch to
be checked out in a clean parent worktree; it runs a foreground Git merge,
waits for a conflicted merge to be committed or aborted, and then performs
protected removal. A configured `commands.close` adapter still runs in the
Head worktree. There is no force option.

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
Hydra. `--json` reports the provider, destination, installed and available
Hydra versions, and a `current` or `updateAvailable` state.

## JSON output

The six read-only data commands above accept `--json`. A successful command
prints one compact JSON object followed by one newline. Every root object has
`"schemaVersion": 1`; field names use camel case, counts and flags retain their
JSON types, and unavailable observations are `null`.

The root payloads are:

| Command | Data |
|---|---|
| `status --json` | `repositoryRoot`, `headsDirectory`, `headCount`, ordered `heads` summaries |
| `head list --json` | ordered `heads` names |
| `head status <NAME> --json` | `name`, recorded intent, observed state, consistency status and issues |
| `head path <NAME> --json` | `name`, validated `path` |
| `doctor storage --json` | backend, primitive, environment, filesystem, guidance, fallback and isolation flags |
| `skill status <PROVIDER> --json` | provider, destination, state, installed and available versions |

JSON mode does not alter inspection, probing, cleanup, or skill validation.
Operational failures remain non-zero, leave stdout empty, and print the normal
actionable diagnostic on stderr. A path that is not valid Unicode cannot be
represented in JSON and is rejected without lossy conversion. Mutating
commands, `repair`, and `completions` do not accept `--json`.

### `hydra skill update <PROVIDER>`

Updates only an unmodified, Hydra-managed copy. Supports mutually exclusive
`--yes` and `--no` automation flags.

### `hydra skill remove <PROVIDER>`

Removes only an unmodified, Hydra-managed copy. Supports mutually exclusive
`--yes` and `--no` automation flags.

## Output and composition notes

- Operational errors use a non-zero exit status and are written to `stderr`.
- After command parsing, operational errors are followed by a `next:` line with
  the relevant safe action. Follow that action before retrying; `hydra repair`
  is suggested only for local Git/Hydra state that it can inspect or reconcile.
- Interactive Head creation progress is written to `stderr`; redirected
  execution omits it.
- `head path` preserves the exact path for non-terminal pipelines while human
  output escapes control characters.
- JSON output is versioned and intended for agents, scripts, and other programs;
  do not parse the human-readable summaries when `--json` is available.
- Inspection commands do not repair or rewrite state.
- Dynamic shell completion suppresses project-discovery errors and returns no
  candidates outside a readable Hydra project.

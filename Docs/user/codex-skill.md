# Optional Codex Skill

Hydra ships a portable Agent Skill that teaches an AI agent to use the Hydra
CLI without bypassing its safety boundaries. Codex is the only currently
supported installation provider.

The skill is optional. Homebrew installs the executable and packaged skill
source, but it never writes to your personal Codex skills directory without a
separate explicit command.

## Install

Run:

```bash
hydra skill install codex
```

In an interactive terminal Hydra displays the resolved destination,
`$HOME/.agents/skills/hydra`, and asks for confirmation with a default-negative
choice. Empty input, unavailable input, or interruption installs nothing.

Automation must state the decision explicitly:

```bash
hydra skill install codex --yes
hydra skill install codex --no
```

The presence of Codex never implies consent.

## Inspect and update

Check whether the installed copy is current and unmodified:

```bash
hydra skill status codex
```

After upgrading the Hydra binary, update the independently managed skill:

```bash
hydra skill update codex
```

For unattended use:

```bash
hydra skill update codex --yes
hydra skill update codex --no
```

Codex normally detects skill changes automatically. Restart the Codex session
only when `$hydra` does not appear or the updated instructions are not visible.

## Remove

Remove only the unmodified copy owned by Hydra:

```bash
hydra skill remove codex
```

Automation can use `--yes` or `--no`. Uninstalling the Homebrew Formula does
not remove the personal skill.

## Ownership protection

Hydra installs a provenance manifest containing provider, Hydra version, and
SHA-256 digests of the canonical files. Update and removal proceed only when
the installed structure and contents exactly match a copy managed by Hydra.

Hydra safely refuses to overwrite or delete:

- an existing skill of unknown origin;
- a symlinked destination or entry;
- a tree with an extra file;
- a missing or invalid provenance manifest;
- a locally modified file or checksum mismatch.

Do not bypass this refusal with recursive deletion or a manual copy. Inspect
and preserve local customizations first.

## Invoke the skill

You can ask Codex explicitly to use it:

```text
Use $hydra to develop this task in an isolated Head based on main.
```

The skill is designed to:

- check the installed Hydra version and help before assuming syntax;
- inspect repository and existing Head state;
- choose a task-specific Head name, source, and target;
- create or select a Head and move work to the path reported by Hydra;
- keep edits and authorized commits on the private Head branch;
- inspect configured open and close commands before executing them;
- leave completed work available for review by default;
- require explicit authorization before integration, forced removal, or other
  destructive actions;
- use `hydra repair` conservatively and preserve ambiguous state.

A new Head starts from the commit selected by `--from`; the skill cannot capture
uncommitted changes from the starting worktree. Commit or otherwise preserve
the intended source state before asking an agent to create a Head from it.

The skill does not automatically run `hydra head close`, force removal, edit
Hydra's local metadata, or delete a worktree manually.

## Canonical artifact

The maintained portable source lives in the repository at `skills/hydra/`.
Provider metadata may improve presentation, but it does not redefine Hydra's
workflow.

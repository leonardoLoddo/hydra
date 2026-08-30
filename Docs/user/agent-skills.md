# Optional Agent Skills

Hydra ships one portable Agent Skill that teaches an AI agent to use the Hydra
CLI without bypassing its safety boundaries. The same canonical artifact is
available through provider-specific installation adapters.

The skill is optional. Homebrew installs the executable and packaged skill
source, but it never writes to a personal agent directory without a separate
explicit command.

## Supported providers

| Provider | Command value | Personal destination |
|---|---|---|
| Codex | `codex` | `$HOME/.agents/skills/hydra` |
| Gemini CLI | `gemini` | `$HOME/.gemini/skills/hydra` |

The destinations are not aliases. Install every provider copy you intend to
use. Each copy contains the same `SKILL.md`; provider-specific metadata may be
ignored by hosts that do not use it.

## Install

Choose the provider explicitly:

```bash
hydra skill install codex
hydra skill install gemini
```

In an interactive terminal Hydra displays the resolved destination and asks
for confirmation with a default-negative choice. Empty input, unavailable
input, or interruption installs nothing.

Automation must state the decision explicitly:

```bash
hydra skill install gemini --yes
hydra skill install gemini --no
```

The presence of an agent application never implies consent.

## Inspect and update

Check whether one provider copy is current and unmodified:

```bash
hydra skill status gemini
```

After upgrading the Hydra binary, update that independently managed copy:

```bash
hydra skill update gemini
```

For unattended use, add `--yes` or `--no`. Gemini CLI can rescan installed
skills with `/skills reload`. Codex normally detects skill changes
automatically; restart it only when the skill or updated instructions are not
visible.

## Remove

Remove only the unmodified copy owned by Hydra:

```bash
hydra skill remove gemini
```

Automation can use `--yes` or `--no`. Uninstalling the Homebrew Formula does
not remove any personal provider copy.

## Ownership protection

Every installed copy has an independent provenance manifest containing its
provider, Hydra version, and SHA-256 digests of the canonical files. Update and
removal proceed only when the installed structure and contents exactly match a
copy managed by the same Hydra provider adapter.

Hydra safely refuses to overwrite or delete:

- an existing skill of unknown origin;
- a skill installed by a different provider adapter;
- a symlinked destination or entry;
- a tree with an extra file;
- a missing or invalid provenance manifest;
- a locally modified file or checksum mismatch.

Do not bypass this refusal with recursive deletion or a manual copy. Inspect
and preserve local customizations first.

## Invoke the skill

Ask the agent explicitly to use Hydra when automatic matching does not select
the skill:

```text
Use the Hydra skill to develop this task in an isolated Head based on main.
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

A new Head starts from the commit selected by `--from`; the skill cannot
capture uncommitted changes from the starting worktree. Commit or otherwise
preserve the intended source state before asking an agent to create a Head
from it.

The skill does not automatically run `hydra head close`, force removal, edit
Hydra's local metadata, or delete a worktree manually.

## Canonical artifact

The maintained portable source lives in the repository at `skills/hydra/`.
Provider adapters install that same tree and add only their own provenance
manifest. Provider metadata may improve presentation, but it does not redefine
Hydra's workflow.

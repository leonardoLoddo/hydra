# CLI Interaction Contract

**Status:** current
**Scope:** command visibility, terminal output, confirmations, and completion semantics
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read for help, command hierarchy, prompts, terminal rendering, exit behavior,
completion, or machine-composable output. Skip for internal-only changes.

## Inherited defaults

Load Head isolation and recoverability from
[hydra-mvp-context.md](hydra-mvp-context.md#default-head-isolation-and-recoverability).
CLI interaction cannot authorize hidden lifecycle actions.

## Command and help boundary

Hydra uses Git terminology: `HEAD`, ref, commit, local branch, and worktree.
Help MUST state purpose, syntax, arguments, meaningful defaults, and copyable
examples. Root and group help expose complete nested command syntax. Only
implemented commands and options are advertised.

The current public command families are `init`, `status`, `head`, `completions`,
`repair`, `doctor storage`, and provider-explicit `skill` management. The `head`
group contains `create`, `list`, `status`, `path`, `open`, `close`, and `remove`.
Do not infer short aliases, JSON output, runtime commands, or future features.
Use the actual Clap hierarchy and targeted `--help` as syntax evidence.

## Output and confirmation

Messages MUST be concise, declarative, and outcome-oriented. Success uses stdout;
operational failures use stderr and non-zero exit status. Output MUST distinguish
completed actions from remaining cleanup or recovery work.

Every operational failure emitted after command parsing MUST pair the diagnostic
with a concise `next:` action on stderr. The action MUST be specific to the typed
failure: correct input or configuration, preserve and inspect ambiguous evidence,
retry the interrupted operation, or run `hydra repair` when local Git/Hydra state
can be reconciled. Do not recommend `repair` for unrelated syntax, provider,
program, or storage problems. A failed `repair` tells the user what prerequisite
to resolve before rerunning it. Clap syntax errors retain their generated usage,
and an explicit default-negative cancellation is not an operational failure.

Warnings for an action that completed with residue MUST state both the completed
outcome and the exact follow-up. Guidance cannot imply that an ambiguous or
report-only state is safe to mutate manually.

Creation reports the concrete absolute path and actual backend only after commit.
Compatible interactive terminals can receive a safe local path hyperlink.
Creation phase progress goes only to interactive stderr, not redirected streams.
Progress observers are informational and cannot interrupt the transaction.

Human-facing paths and persisted values MUST neutralize terminal control characters.
`head path` is the explicit machine-composition exception: non-terminal stdout
preserves the exact validated path plus a newline; terminal output remains escaped.
Do not add summaries to that path-only contract.

Informational summaries do not require confirmation. Full-copy overlay cost and
persistent unsafe-symlink exclusion use distinct default-negative prompts defined
in [configuration-and-overlays.md](configuration-and-overlays.md). Only trimmed,
case-insensitive `y` or `yes` confirms those prompts; EOF and other answers cancel.
Skill installation has its own explicit provider and automation choices, governed
by [../development/release-distribution.md](../development/release-distribution.md).
Never infer installation consent from host presence or non-interactive input.

## Shell completion

`hydra completions <shell>` provides Bash, Zsh, and Fish registration. Existing-Head
arguments for `status`, `path`, `open`, `close`, and `remove` offer known names;
`create` does not suggest occupied names. Candidates MUST be ordered, deduplicated,
read-only, prompt-free, and fast enough for interactive use. Outside an initialized
project or with unreadable state, candidate discovery returns no names silently.
Shell adapters own escaping; inventory selection logic is shared.

Homebrew registers completion in package-manager directories without editing
personal shell profiles. The portable Windows ZIP includes a Bash registration;
activation in Git Bash remains explicit. Generated registration remains the fallback
for other installation methods. Internal testable candidate commands are not public
help or new user-facing workflows.

## Evidence and verification

Inspect `crates/hydra-cli/src/main.rs`, `guidance.rs`, `head_create.rs`,
`inspection.rs`, and `output.rs` under that source root. Run CLI `cli_contract`, `head_inspection`,
`completions`, and affected command targets. Check help matches implemented parsers, raw path
output remains composable, redirected output contains no terminal sequences or
progress, and invalid completion state produces no mutations or prompts.
Completion mechanics belong to
[../architecture/shell-completions.md](../architecture/shell-completions.md).

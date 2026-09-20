---
name: hydra
description: "Use Hydra for Git-native isolated Heads and parallel workflows, especially when multiple implementations are developed at the same time. Use its scoped Arts only in their cases: Arena compares materially different implementations, Augury tests consequential uncertainty with a disposable experiment, and Gauntlet adversarially validates existing work. Also trigger for independent working trees and indexes, disposable exploration, safe Head lifecycle management, or evidence from isolated realities."
---

# Hydra

Use Hydra as the workspace boundary for one task. Keep Git as the source of
truth and use only documented Hydra commands for Head lifecycle operations.

## Hydra Arts

Hydra Arts are adaptive strategies built on isolated, disposable Heads. They
may be explicitly requested, suggested when their expected value is plausible,
or invoked autonomously when prerequisites are clear, uncertainty or risk is
materially reduced, and the extra work is proportionate. If the value is
uncertain, suggest the Art before running it.

Hydra Arts are intent-driven, not step-driven. Use the lightest execution that
preserves the Art's intent and invariants. Skip actions that do not materially
improve the result. Never invoke an Art ceremonially or perform a step only
because it appears in an example.

- **Arena — Competitive Implementation:** use when at least two materially
  different implementations are credible and real comparison would resolve an
  important trade-off. Read [references/arena.md](references/arena.md).
- **Augury — Experimental Design:** use during design or brainstorming when a
  disposable experiment can test important assumptions more cheaply than more
  theory. Read [references/augury.md](references/augury.md).
- **Gauntlet — Adversarial Validation:** use on an existing implementation when
  stronger confidence requires active attempts to expose defects, weak tests,
  risky assumptions, or needless complexity. Read
  [references/gauntlet.md](references/gauntlet.md).

Arts may compose when evidence justifies it, but they are not a mandatory
sequence. Preserve all authorization and safety boundaries below. An Art does
not itself authorize integration, target-ref mutation, forced removal, or
discarding identified work unless the user's request clearly includes that
action.

On native Windows, operate the native `hydra.exe` through Git Bash with Git for
Windows on `PATH`. Treat copy-on-write as volume-dependent: compatible ReFS
volumes may use block cloning, while NTFS and unsupported volumes use isolated
full copies. Run `hydra doctor storage` before making storage-cost assumptions.
When native Windows initialization, storage diagnosis, or Head creation reports
full copy, follow the printed setup URL; do not treat the guide as proof until
a new probe reports `Windows ReFS block clone`.
Do not create a Head containing tracked or selected overlay symlinks on Windows;
that materialization remains unsupported.

Inside WSL 2, Hydra uses Linux reflinks rather than Windows block cloning. The
default ext4 root and Windows interop mounts may require full copy. When
initialization, diagnosis, or Head creation prints the WSL setup URL, use it to
prepare a fresh clone on a reflink-capable Linux volume and accept CoW only
after `hydra doctor storage` reports `Linux reflink`.

When the user asks to install Hydra on native Windows, use the checksummed
stable Windows ZIP linked from the repository README only after it resolves to
a published GitHub Release. Do not use expiring CI artifacts or binaries
committed to the source tree. If no Windows release asset exists yet, report
that boundary and use a source build only when the user accepts the Rust
toolchain requirement.
The portable ZIP includes `completions/hydra.bash` but does not edit the user's
Git Bash profile. When completion is requested, source that packaged file from
`~/.bashrc`; if it is unavailable, use `source <(hydra completions bash)`.
Never modify a user's shell profile without explicit authorization.

## Establish the starting state

1. Run `hydra --version` and `hydra --help`. Never assume an installed build
   supports a command or option that its help does not show.
   Homebrew installs Bash, Zsh, and Fish completion files automatically. If
   completion is unavailable, do not edit a user's shell profile without
   authorization; use `hydra completions <shell>` as the documented fallback.
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
   If initialization was interrupted, rerun the same `hydra init [PATH]`.
   Hydra resumes only from its unlocked, internally consistent journal and
   byte-identical existing artifacts. Wait when initialization is active, and
   preserve every named path when the journal or partial state is invalid or
   mismatched; never edit or delete the journal or partial directories by hand.
5. Run `hydra head list` and decide whether this task should resume an existing
   Head or create a new one. Never reuse a Head merely because its name looks
   related: inspect it with `hydra head status <name>` and stop if its ownership,
   task, branch, path, or existing work is unclear.

Hydra lifecycle commands generally may run from the main project worktree or
any managed Head. `hydra head close` is the exception: it must run from the
canonical parent project worktree. Hydra uses the shared locator to normalize
other lifecycle invocations to the canonical parent project, even when
`.hydra.json` is missing or stale in the calling Head. Configuration, Git
defaults, overlays, project reporting, and inventory therefore behave exactly
as if the command ran from the parent.
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

When the installed CLI advertises `--dry-run`, use it before creation when the
resolved base, target, destination, overlay cost, or storage policy needs review:

```bash
hydra head create <name> --from <source> --target <target> --dry-run --json
```

Treat the plan as a current snapshot, not a reservation. It makes no durable
change; the real create replans and can still fail if state changes.

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
- If Hydra reports that full copy is required, report the file count, byte
  size, and any platform setup URL before answering. Follow the guide only for
  diagnosis; answer yes only after the user authorizes that storage cost.
- A negative answer or EOF is a safe cancellation. Do not bypass either prompt
  with manual copying, Git worktree commands, or metadata edits.

If creation fails, do not substitute `git worktree add`, manual directory
copying, or metadata edits. Inspect `hydra status`, `git worktree list`, and the
reported error. Follow Hydra's `next:` action when present: it is selected from
the typed failure. Run `hydra repair` when instructed for lifecycle residue,
then stop if repair leaves ownership or state ambiguous.

If Hydra reports failure while normalizing the target ref, verify that the
explicit `--target` names an existing local branch. If it reports a versioned
directory-policy mismatch, do not create the configured directory or edit the
locator; restore the reviewed project configuration or stop for user guidance.

## Move all task work into the Head

1. Obtain the authoritative directory with `hydra head path <name>`.
2. Set every task, edit, build, and test command's working directory to that
   exact path. Lifecycle commands may still be launched there because Hydra
   resolves their control context to the parent project, except `head close`,
   which must be launched from the canonical parent worktree.
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
is clean. Change the command's working directory to the canonical parent
project, verify that the Head's recorded target branch is checked out there,
and verify that parent worktree and index are clean. Hydra runs a normal Git
merge there and inherits Git's terminal output.

When `--dry-run` is available, prefer `hydra head close <name> --dry-run
--json` to validate and report the current native classification or expanded
adapter before requesting or exercising integration authority. A native
`mergeCommit` plan does not predict conflicts. The preflight never merges,
executes the adapter, or removes the Head.

If Git reports conflicts, keep Hydra running. Resolve the files and commit the
merge in the parent worktree through the IDE or another terminal. Hydra
validates the resulting commit and automatically resumes protected removal.
Run `git merge --abort` to abort the Hydra close and preserve the Head. Do not
start a different Git operation or replace the expected merge while Hydra is
waiting.

If `commands.close` uses a custom command, still invoke close from the canonical
parent project. Report its program, arguments, and `removeOnSuccess` policy
before execution. Hydra starts the adapter in the Head worktree. Treat the
command as trusted project code that is not sandboxed and may push branches,
create pull requests, modify files, or contact services. Execute it only when
the user's authorization covers those concrete effects. Do not describe a
successful custom command as an integration unless its observed result proves
that claim.

For native close, a target worktree with staged, modified, deleted, or
untracked files, or with a Git operation in progress, blocks close without
mutating the target or Head. Report the blocker; do not switch branches, stash
another task's files, or alter another worktree to force integration. Report
Hydra's integration strategy and result after a successful close.

Use `hydra head remove <name>` only for an authorized removal after inspecting
the Head. Never use `--force` unless the user explicitly authorizes discarding
the identified uncommitted files. Force removal must still preserve commits
that are not integrated, and its output must be reported.

## Handle inconsistencies safely

- Treat an emitted `next:` line as the first recovery action for that exact
  failure. Do not replace specific input, configuration, provider, program, or
  storage guidance with a generic repair attempt. If `hydra repair` itself
  fails, resolve the reported prerequisite and rerun it.
- Treat `hydra status`, `hydra head status <name>`, `hydra head list`, and
  `hydra head path <name>` as the read-only inspection surface.
- When inspection data will be parsed by an agent, script, or tool, prefer
  `--json` on `hydra status`, `hydra head list`, `hydra head status <name>`,
  `hydra head path <name>`, `hydra doctor storage`, `hydra skill status
  <provider>`, and the `head create` or `head close` preflight. Expect one
  object with `schemaVersion: 1` and a trailing newline.
  Do not parse human summaries when versioned JSON is available. JSON mode does
  not authorize mutation; create and close require `--dry-run` with `--json`,
  while `repair`, other lifecycle mutations, and completions do not accept it.
  Treat a non-zero exit with empty stdout and a `next:` diagnostic on
  stderr as an operational failure, not as an incomplete JSON response.
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
  diagnosis. Report its backend, native primitive, environment, and filesystem,
  plus any temporary path that Hydra could not clean up; do not remove such a
  path blindly. Treat WSL and native Windows full-copy URLs as setup
  information, not as proof of CoW. Use the Windows guide to prepare a new ReFS
  Dev Drive layout and accept CoW only after the real probe reports `Windows
  ReFS block clone`. For a new WSL setup, keep a fresh project clone and sibling
  Heads directory on the same reflink-capable Linux volume and accept CoW only
  after the real probe reports `Linux reflink`. Never create, format, mount,
  resize, or relocate a volume without explicit user authorization; do not edit
  Hydra's locator to move an existing installation.
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
- For an interrupted `head create`, run `hydra repair`. Authorize cleanup only
  when Hydra identifies either no worktree/path residue or one exact incomplete
  worktree at the recorded managed path and private branch. Hydra revalidates
  under lock, removes only that confirmed incomplete worktree, and deletes an
  unchanged ref with compare-and-swap. Authorize adoption of a finalized Head
  only when Hydra reports it recoverable from matching journal, worktree, ref,
  base commit, clean status, and any other recovery records. Preserve and
  report mismatches, dirty state, or advanced branches; never edit or delete
  journals, branches, recovery records, or directories manually.

Preserve recoverability over convenience: leave the Head and its private branch
in place whenever safe integration or removal cannot be proven.

## Manage this skill only when requested

Do not update or remove the active Hydra skill merely because a newer Hydra
binary exists. When the user explicitly asks to manage it, identify the host
provider and use `hydra skill status <provider>` first. The supported values are
`codex`, `gemini`, `agy`, and `antigravity`. Inspect `hydra skill --help` before
assuming the installed binary supports any one. Use
`hydra skill update <provider>` or `hydra skill remove <provider>` only after
the user confirms the resolved destination and action. Hydra preserves unknown,
provider-mismatched, or locally modified skill content; do not bypass that
refusal with a manual copy or recursive deletion.

Gemini CLI may discover the Codex adapter's copy through its
`$HOME/.agents/skills` alias. Discovery does not change ownership: manage that
copy with `codex`, not `gemini`. Use `gemini` only for the independent
`$HOME/.gemini/skills/hydra` destination.

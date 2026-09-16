# Head Lifecycle Contract

**Status:** current
**Scope:** initialization, creation, inspection, configured adapters, integration, and removal
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read when changing a Head lifecycle command, source or target semantics, adapter
execution, or destructive-action policy. Skip for independent packaging and storage changes.

## Inherited defaults

Load Head isolation and recoverability from
[hydra-mvp-context.md](hydra-mvp-context.md#default-head-isolation-and-recoverability).
This leaf defines explicit lifecycle actions within that safety boundary.

## Default: canonical parent context

**Applies to:** project discovery, configuration, Git defaults, overlays, ownership,
and reporting for initialized Hydra lifecycle commands.

Hydra resolves the shared locator through the caller's Git common directory and
validates the canonical parent project. Calling from a managed Head MUST use the
same parent policy and defaults. A Head without `.hydra.json` remains operable;
its private branch or local files never become implicit inputs. Creating from a
Head creates a sibling, and re-running `init` reports the parent already initialized.

### Close caller exception

**Modifies:** canonical parent context.
**Applies to:** `hydra head close`, including custom adapters.
**Effect:** extends the default with a caller restriction.

Close MUST originate in the canonical parent worktree or its subdirectories.
A call from a Head is rejected before adapter execution or mutation and reports
the parent path. Native close additionally requires the recorded target branch
checked out in that parent. This is the approved 1.x close boundary, not a
checkout-free integration workflow.

## Initialization and creation

`hydra init [PATH]` defaults to the current directory, resolves Git root and common
directory, validates external Heads placement, persists shared policy and local
ownership, probes storage, and reports the backend. It MUST NOT change application
files. Reuse of an existing installation is allowed only for the exact empty,
owned state described in
[../architecture/project-initialization.md](../architecture/project-initialization.md).
Missing policy for existing Heads MUST NOT be replaced with guessed defaults.

`hydra head create <name> [--from <ref>] [--target <ref>]` resolves source to an
exact commit, creates a private branch and worktree, initializes its index,
materializes tracked content and overlays, verifies the result, and registers
metadata before reporting success.

Without `--from`, use the parent `HEAD`. Without `--target`, a local source branch
is also the target. A detached commit, tag, or other non-local source requires
an explicit existing local target. Explicit `--target` remains authoritative.
Duplicate names, private branches, or destinations MUST fail before mutation.
Source edits during copying MUST cause safe failure when destination identity
no longer matches the plan.

`head create --dry-run` validates and reports the current creation plan without
creating a mutation lock, branch, worktree, inventory entry, prompt, or policy
change. The plan is a snapshot and MUST be revalidated by a later create.

## Inspection

`status`, `head list`, `head status`, and `head path` are read-only. They MUST NOT
acquire mutation locks, rewrite state, or repair implicitly. Safe inconsistencies
remain reportable; unsafe recorded paths and invalid metadata are errors.

List returns stable ordered names; path returns only the validated absolute
recorded path. Status reports the canonical parent and `clean`, `modified`, or
`inconsistent` Heads. Detailed status shows observed worktree branch, commit,
changes, recorded base and target, presence, and inconsistencies. When the observed
branch differs, report both observed and expected refs.

Ahead/behind compares the observed worktree commit with the current symbolic
base, or recorded `baseCommit` for a non-symbolic source. A missing symbolic base
is reported and falls back to `baseCommit`. Later refs cannot reinterpret an
original abbreviated commit expression.

## Configured processes

`head open` requires `commands.open`; Hydra chooses no implicit editor.
Open and custom close specify `program` and `args` separately. Supported
placeholders are `{name}`, `{path}`, `{headRef}`, `{baseRef}`, and `{targetRef}`.
Hydra MUST reject malformed or unsupported templates and NUL values before launch,
and MUST NOT construct a shell command from expanded values.

Adapters run in the validated selected Head path and inherit standard streams.
Open can operate on a dirty Head and does not mutate Hydra state. Custom close
requires a clean, consistent Head. Trusted programs can make external changes;
Hydra MUST NOT claim to sandbox or safely undo arbitrary adapter effects.

## Close and protected removal

Absent custom configuration, close runs a foreground `git merge --no-edit` of
the validated Head commit in the clean parent target worktree: fast-forward when
possible, otherwise a merge commit. It does not rebase, squash, or resolve conflicts.
A dirty target or Git operation in progress blocks integration without mutation.

Conflicts remain normal Git state in the parent. Hydra waits without holding a
project mutation lock. A clean resolution commit must have exactly the recorded
target and Head snapshots as parents, in that order, before removal resumes.
`git merge --abort` preserves the Head and aborts close. Unexpected parents, dirty
completion, or a different Git operation stop the workflow without removal.

A `commands.close` strategy of `command` replaces native integration with the
configured program. Custom configuration MUST explicitly supply `removeOnSuccess`; false preserves
the Head. Absent `commands.close`, native merge includes protected removal.
Successful adapters are not assumed to have integrated commits. Their optional
removal uses ordinary protection and cannot bypass unintegrated-work checks.
Failed adapters preserve the Head and report observed target changes without
attempting arbitrary rollback. Success followed by failed removal reports both
phases separately and preserves recoverable state.

`head remove` refuses dirty, untracked, unintegrated, missing, or inconsistent
Heads. Worktree removal alone MUST NOT authorize deleting recoverable commits.

**Force exception:** `head remove --force` replaces two ordinary preconditions for
worktree removal: the worktree must be clean, and its commits must already be
integrated. It explicitly authorizes discarding tracked, staged, and untracked
changes and removing an unintegrated Head's worktree and inventory entry while
preserving its private ref. The preserved ref's name MUST be reported.

This exception does not bypass ownership, path, branch, registration, or target
validation. It never waives the integration requirement for private-branch deletion:
delete that branch only after current target reachability and expected-object
comparison protect it. Close does not silently force removal.

## Evidence and verification

Inspect `crates/hydra-core/src/head/close.rs`, `head/open.rs`, `head/removal.rs`,
and `head/inspection.rs` under the same source root. CLI `head_close`, `head_open`,
`head_remove`, `head_inspection`, `init_success`, and `head_create_success` tests
exercise these contracts using disposable repositories. Check actual refs,
worktrees, metadata, refused actions, valid merge parents, and abort preservation.
Select the corresponding Architecture leaves for transaction and failure details.

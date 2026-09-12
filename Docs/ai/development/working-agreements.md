# Repository Working Agreements

**Status:** current
**Scope:** planning, authorization, evidence, and completion of repository changes
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read before a non-trivial repository change or engineering decision. Skip for
read-only lookup that makes no new decision and changes no repository artifact.
Rust mechanics, commit format, and Hydra skill packaging remain independently
selected through this domain router.

## Default: complete scoped change

**Applies to:** every non-trivial repository change.

1. Read routed knowledge and inspect existing code, tests, configuration, and work.
2. Classify all concerns: bug fix, product or backend feature, CLI, Git/worktree,
   filesystem/materialization, configuration/state, documentation, refactor,
   build/tooling/dependency, and commit/history. Categories are cumulative.
3. Define observable acceptance criteria, failure behavior, recovery, edge cases,
   affected consumers, and likely regressions before implementation.
4. Use the smallest complete change consistent with approved scope and safety.
5. Verify implementation, then perform the Governance knowledge-impact check.
6. Report concrete results, regression evidence, documentation impact, and limits.

Do not introduce an abstraction, dependency, service, crate, or runtime until
existing project patterns have been inspected and shown insufficient. Do not
expand the roadmap to make an otherwise complete change appear more comprehensive.

## Inherited engineering rules

For changes that affect compiled, runtime, CLI, persisted-state, error, or platform
behavior, load [rust-tdd-context.md](rust-tdd-context.md). Its test-first workflow
and quality gates extend the complete-scoped-change default.

Production application and core libraries MUST remain Rust. Another core runtime
or language requires an explicit architectural decision and user approval.
An unavoidable TDD deviation requires explicit user authorization before
implementation and must be reported. `unsafe` requires demonstrated need,
a documented safety contract, focused boundary tests, regression assessment, and
compatibility with the repository's currently forbidding lint configuration.
Do not weaken that lint merely to make an implementation possible.

## Evidence and uncertainty

Normative authority and descriptive evidence are separate. Approved scope and
contracts define intended behavior; source, tests, runtime, state, manifests, and
history describe what exists. When these disagree, identify sources and scope,
stop the affected decision, and preserve unrelated work. Do not encode an
unresolved assumption as current policy.

Verify exact external APIs, versions, flags, and platform behavior with official
documentation when local evidence is insufficient. Before dependency changes,
check standard-library and existing-dependency alternatives, portability,
maintenance, installation, security, and native-build costs. Never invent an API
or supported platform from memory when it can be verified.

## High-risk changes

Treat creation, removal, integration, repair, ref mutation, path validation,
symlinks, storage fallback, overlays, persistence, schema compatibility,
interruption, cleanup, command execution, user input, and shared services as
high risk. Assess affected refs, paths, state, commands, consumers, and irreversible
failure modes. Verify both the changed contract and neighboring behavior.

A shared abstraction changes only when the root cause belongs there, meaningful
consumers have been inspected, the broader behavior is intentional, and
representative regression coverage exists. Prefer local adaptation and existing
interfaces to speculative shared flexibility. Avoid unrelated formatting churn.

Expected input, Git, filesystem, configuration, and recoverable operational failures
MUST return contextual errors, preserve sources, and avoid unnecessary sensitive
path or command exposure. Do not panic or silently choose a less safe backend.

## Git and filesystem operations

Resolve and validate roots, common directories, refs, paths, and ownership before
mutation. Prefer atomic and recoverable operations. Preserve enough evidence to
diagnose interruption; remove only artifacts proven owned by the operation.

Destructive tests MUST use newly created temporary repositories and directories,
never the source checkout, real user projects, or existing Heads. Avoid broad
recursive targets, unresolved globs, and unvalidated variables. Pass process
arguments separately rather than constructing unescaped shell commands.
Mutable hard links MUST NOT be used for isolation. Worktree removal does not by
itself authorize branch deletion. Destructive or ambiguous repair requires explicit
confirmation; uncertainty is not permission to edit ownership or discard data.

## Configuration and compatibility

Treat versioned configuration and local state as compatibility-sensitive external
input. Validate at trust boundaries and use atomic writes. Define behavior for
missing, malformed, stale, and newer versions. Preserve supported schemas or
provide an explicit migration. Persist derived values only for an established
reconciliation, performance, or behavior need. Git-unrecoverable work MUST NOT
depend solely on Hydra metadata.

## Commit authorization

Read [commit-conventions.md](commit-conventions.md) before creating, amending,
squashing, reviewing, or proposing a commit. Do not create commits without user
authorization. Review the actual staged diff and exclude unrelated user changes.
Do not rewrite shared history or bypass hooks and required checks without explicit
authorization. One logical change includes required behavior, regression tests,
and documentation together.

## Documentation and skill projection

After verification, inspect the actual diff for changed behavior, architecture,
terminology, workflow, dependency, invariant, operational procedure, or knowledge.
Use Governance for admission, canonical ownership, authoring, routes, and validation.

For an explicit user-documentation task or any user-visible CLI, configuration,
workflow, validation, output, or recovery change, open [the user router](../../user/user-context-router.md). Update the affected
English pages rooted at [the English guide](../../user/hydra-user-guide.md) and
[the maintained Italian guide](../../user/hydra-user-guide.it.md) in the same change.
Examples MUST match implemented help and tests. Keep both languages navigable and
complete; include customization safety and current limits. Never advertise planned
syntax or instruct users to edit local metadata or perform unsafe manual recovery.

Every non-trivial task also assesses `skills/hydra/`. For an agent-operable change,
load [hydra-skill-context.md](hydra-skill-context.md) and synchronize the canonical
skill in the same task. The product, implementation, CLI help, English and Italian
guides, and operational skill MUST agree. A documentation-only governance change
needs no artificial Hydra skill wording edit when it changes no Hydra operation.

## Completion verification

Confirm the changed behavior is directly exercised; adjacent regressions and shared
consumers are protected; destructive failure paths preserve the documented state;
and no test was weakened, deleted, ignored, or broadened just to pass.
Report focused and full applicable checks, TDD evidence, documentation and skill
impact, and unverified platforms or assumptions. A broad green suite alone is
insufficient. Pure documentation changes use structural, semantic, routing, and
operational checks rather than artificial Rust tests.

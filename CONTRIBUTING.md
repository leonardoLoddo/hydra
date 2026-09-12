# Contributing to Hydra

Hydra is an early preview that manages Git worktrees, branches, and filesystem
state. Contributions should be small, evidence-backed, and especially careful
around recovery and destructive behavior.

## Report a problem

Use the repository's GitHub issue chooser and select either the bug report or
preview feedback form. Search existing issues first and remove credentials,
private repository contents, and personal paths from logs or reproductions.

Security-sensitive defects must follow [SECURITY.md](SECURITY.md) instead of a
public issue.

## Prepare a change

1. Read [AGENTS.md](AGENTS.md) and start from the
   [LibrAIrian project knowledge router](Docs/ai/ROUTER.md).
2. Confirm the behavior belongs to the current MVP and inspect the existing
   implementation and tests before proposing a design.
3. For every behavior change, follow Red-Green-Refactor: observe the focused
   test fail for the expected reason before writing production code.
4. Use disposable temporary repositories and directories for Git or
   filesystem lifecycle tests. Never test destructive behavior against the
   Hydra checkout or another real project.
5. Update routed product, architecture, user, and skill documentation whenever
   the contract visible to users or agents changes.

Hydra production code is Rust and uses the repository-pinned toolchain. Avoid
new dependencies or abstractions until existing project patterns have been
shown insufficient.

## Verify the change

For production changes, run the focused tests first, then the complete quality gates:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Run platform-specific, all-feature, packaging, or disposable-repository tests
when the affected contract requires them. If a relevant check cannot run,
state the limitation clearly in the pull request.

After implementation verification, perform the LibrAIrian knowledge-impact check.
Follow the [Governance route](Docs/ai/governance/ROUTER.md) for canonical ownership,
authoring, runnable link and graph checks, representative routing cases, and
fallback validation. Update affected knowledge with the implementation. A pure
documentation change uses these checks instead of artificial Rust tests; no-update
conclusions need a concrete reason. Report code verification and knowledge
verification separately.

## Commit and pull request

Hydra uses English
[Conventional Commits](Docs/ai/development/commit-conventions.md).
Keep each commit focused and include its production code, TDD tests, and
required documentation together.

A pull request should explain:

- the problem and intended observable behavior;
- the Red and Green evidence for runtime changes;
- regression risks and the adjacent behavior verified;
- documentation and Hydra skill impact;
- platform or test limitations that remain.

Do not weaken, delete, broaden, or ignore meaningful tests merely to make a
change pass. Do not rewrite shared history or bypass required checks.

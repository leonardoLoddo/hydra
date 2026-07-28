# AGENTS.md

## Purpose

This file defines the operating rules for AI agents working on Hydra.
These rules are binding unless they conflict with higher-priority platform instructions or explicit user requirements.

Hydra is at an early stage. Agents must preserve documented product intent while allowing implementation conventions to emerge from verified code and tests rather than speculation.

---

# Scope and Priorities

1. Follow routed project documentation and explicit user requirements.
2. Preserve Hydra's safety, recoverability, Git compatibility, and isolation guarantees.
3. Develop all production behavior in Rust using mandatory test-driven development.
4. Preserve existing behavior through explicit regression assessment and targeted tests.
5. Inspect the real repository before making implementation assumptions.
6. Deliver the smallest complete and verifiable change.
7. Keep reusable documentation aligned with behavior.

---

# Source of Truth Order

When resolving uncertainty, use this priority order:

1. Explicit requirements in the current task.
2. Project documentation routed from [`Docs/hydra-context-router.md`](Docs/hydra-context-router.md).
3. Existing code, tests, configuration, and observed runtime behavior.
4. Official documentation for the exact tools and versions in use.
5. Generic ecosystem knowledge.

Lower-priority sources must not silently override higher-priority sources.

If routed documentation conflicts with code or observed behavior, stop and report the conflict. Do not decide silently which one is correct.

---

# Mandatory Documentation Routing

Before proposing or implementing a solution, the agent MUST:

1. Open [`Docs/hydra-context-router.md`](Docs/hydra-context-router.md).
2. Classify all documentation domains touched by the task.
3. Follow every applicable child router until the required leaf documents have been identified and consulted.
4. Treat routes as cumulative when a task spans multiple domains.

The macro-router and its child routers are the sole source of truth for documentation paths and consultation triggers.

- Do not place leaf-document routes in this file.
- Do not infer a document path from memory when the router can provide it.
- If no route clearly covers the task, inspect the relevant code and identify whether a documentation gap exists.
- If ambiguity would materially change product behavior, architecture, safety, compatibility, or scope, ask for clarification before proceeding.

---

# Documentation Architecture

Project documentation is organized by domain under `Docs/`.

Each documentation subfolder MUST contain a context router that:

- explains the domain's purpose and boundaries;
- describes the nature of each direct child document and nested router;
- states exactly when each route must be consulted;
- uses relative links;
- keeps routing metadata separate from source-of-truth content.

When documentation is created, renamed, moved, or removed, update its owning router and every affected ancestor router in the same change.

The canonical topology and maintenance rules live in [`Docs/hydra-context-router.md`](Docs/hydra-context-router.md).

---

# Documentation Quality Rules

Documentation is a reusable knowledge system, not a task log.

Document:

- product invariants and behavior contracts;
- architectural decisions and component boundaries;
- decision rules and rationale;
- security, filesystem, Git, and recovery constraints;
- stable CLI or configuration contracts;
- reusable development and testing conventions.

Do not document:

- one-off implementation steps;
- temporary plans or investigation notes;
- changelog-style descriptions of a single task;
- local details that do not establish a reusable rule;
- speculative architecture with no current requirement.

A useful documentation rule answers:

- What must future contributors do?
- When does the rule apply?
- Why does the rule exist?

---

# Task Classification

Before making changes, classify the task using one or more of these categories:

- bug fix;
- product or backend feature;
- CLI behavior;
- Git/worktree behavior;
- filesystem or materialization behavior;
- configuration or state change;
- documentation update;
- refactor;
- build, tooling, or dependency change;
- commit or history operation.

Classification is cumulative. Use it to select documentation routes, assess risk, and choose verification.

---

# Working Method

For every non-trivial change:

1. Read the applicable documentation.
2. Inspect related code, tests, configuration, and repository conventions.
3. Define the expected behavior and acceptance criteria.
4. Identify edge cases and regression surfaces.
5. Define the tests that will prove the new behavior and protect likely regressions.
6. Follow the mandatory Red-Green-Refactor workflow.
7. Run focused checks, then the complete Rust quality gates.
8. Assess documentation impact and update routed documentation when reusable behavior or contracts changed.
9. Report the result, TDD evidence, regression protection, and any remaining limitations.

Do not introduce a new abstraction, dependency, service, or architectural layer until existing project patterns have been checked and found insufficient.

---

# Rust Implementation Rules

Hydra's production application and core libraries MUST be implemented in Rust.

- Use the Rust toolchain and edition pinned or declared by the repository.
- Use Cargo as the build, dependency, test, and workspace system.
- Prefer safe Rust and the standard library when they provide a clear and maintainable solution.
- Keep platform-specific filesystem behavior behind narrow, testable boundaries.
- Do not introduce another application runtime or language for core product behavior without an explicit architectural decision and user approval.
- Treat `unsafe` code as high risk. It requires a demonstrated need, a documented safety contract, focused tests around the boundary, and explicit regression assessment.
- Do not panic on expected user input, Git state, filesystem state, or recoverable operational failures. Return contextual errors instead.
- Preserve error sources and make failures actionable without exposing sensitive path or command data unnecessarily.

Detailed Rust, TDD, and quality-gate rules are selected through the documentation router and are binding for every code change.

---

# Commit Integrity

Before creating, amending, squashing, reviewing, or proposing a commit, follow the development route selected by the documentation macro-router.

- Hydra uses Conventional Commits with English messages.
- Each commit must represent one coherent logical change.
- Include production code, its TDD regression tests, and required documentation in the same logical commit.
- Review the staged diff and exclude unrelated user changes.
- Do not create WIP or vague commits in history intended for integration.
- Do not rewrite shared history or bypass hooks and required checks without explicit authorization.
- Do not create commits unless the user has requested or authorized the commit operation.

The routed commit convention owns the allowed types, preferred scopes, description rules, breaking-change format, and message examples.

---

# Mandatory Test-Driven Development

Every change that can affect compiled behavior, runtime behavior, CLI behavior, persisted state, error behavior, or platform behavior MUST follow test-driven development.

The required cycle is:

1. **Red**: write or update an automated test that expresses one required behavior or regression contract.
2. Run the narrowest relevant test and observe it fail for the expected reason. A compilation failure caused by the intentionally missing behavior is a valid initial Red state only when it proves the test reaches the missing contract.
3. **Green**: write the minimum production code needed to pass that test.
4. Run the focused test and observe it pass.
5. **Refactor**: improve the implementation only while the relevant tests remain green.
6. Repeat the cycle for the next behavior.
7. Run the complete applicable regression and Rust quality gates before completion.

Production behavior MUST NOT be implemented before its failing test has been observed.

Additional rules:

- A bug fix starts with a regression test that reproduces the defect.
- A refactor starts with existing behavioral coverage; if coverage is insufficient, add characterization tests before changing the implementation.
- A shared abstraction change requires tests for the target behavior and representative existing consumers.
- Tests written after the implementation do not satisfy the TDD requirement.
- Pure documentation changes are not production behavior and do not require artificial Rust tests; they still require link and consistency validation.
- Any unavoidable deviation from TDD requires explicit user authorization before implementation and must be reported in the final result.

---

# Hydra Safety Invariants

The detailed product contracts are owned by the routed documentation. During all implementation and testing, agents must pay particular attention to these non-negotiable classes of risk:

- loss of uncommitted work, commits, branches, worktrees, or local files;
- unintended changes to the user's active repository or another Head;
- violation of working-tree, index, branch, or filesystem isolation;
- unsafe path traversal or symlink handling;
- shell or command injection;
- partial state left by interrupted multi-step operations;
- metadata changes that cannot be reconciled with Git;
- silent fallback to a less safe storage mechanism.

Never weaken a documented safety guarantee for convenience or performance.

---

# Git and Filesystem Operations

Hydra manages Git repositories and filesystem trees, so destructive operations require a higher standard of care.

- Resolve and validate paths, repository roots, Git common directories, refs, and ownership before mutation.
- Never assume the current working directory is a safe test target.
- Use newly created temporary repositories and temporary directories for destructive lifecycle tests.
- Never run destructive integration tests against the Hydra source repository, a user's real project, or an existing Head.
- Avoid unresolved globs, broad recursive targets, and unvalidated environment variables in destructive commands.
- Prefer recoverable and atomic operations.
- Preserve enough state to diagnose and reconcile interrupted operations.
- Never use mutable hard links as a shortcut for isolated files.
- Do not delete a branch merely because its worktree is being removed.
- Destructive or ambiguous repair actions require explicit user confirmation.

Commands that execute Git or user-configured programs must pass arguments safely and must not build an unescaped shell command from user input.

---

# Product and Feature Work

Before implementing product behavior:

1. Follow the product route and confirm the behavior belongs to the current scope.
2. Identify which user-visible and safety contracts are affected.
3. Inspect the existing implementation and its tests.
4. Define success, failure, rollback, and recovery behavior.
5. Prefer explicit actions over implicit Git, filesystem, network, or process side effects.
6. Add verification at the lowest level that proves the contract, including real temporary Git repositories when integration behavior matters.

Do not expand the roadmap or implement out-of-scope features merely because they make the current design appear more complete.

---

# Bug Fixing

Before changing code for a bug:

1. Reproduce the issue or collect direct evidence of the failure.
2. Trace the relevant Git, filesystem, configuration, and state transitions.
3. Identify the root cause and likely adjacent regressions.
4. Add a failing regression test that reproduces the defect.
5. Apply the narrowest fix that restores the documented contract.
6. Verify both the target behavior and the most likely neighboring behavior.

Do not implement a speculative fix without evidence.

---

# Configuration and State Changes

Treat changes to versioned project configuration and local Hydra state as compatibility-sensitive.

- Validate all external data at the trust boundary.
- Preserve or explicitly migrate supported schema versions.
- Use atomic writes for state that must not be partially persisted.
- Define behavior for missing, malformed, stale, or newer-version state.
- Do not store derived data unless it is needed for reconciliation, performance, or an explicit product contract.
- Never make Git-unrecoverable state depend solely on Hydra metadata.

Schema or compatibility decisions must be documented in the appropriate routed domain once that domain exists.

---

# Regression Risk Assessment

Assess regression risk before implementation.

The assessment must identify:

- existing behavior that could break;
- affected Git refs, paths, state files, commands, or user flows;
- destructive or irreversible failure modes;
- whether shared abstractions or cross-command behavior are touched;
- the verification needed for the identified risk.

Treat changes as high risk when they affect:

- Head creation, removal, repair, or reconciliation;
- branch/ref selection or mutation;
- path validation, traversal, or symlink behavior;
- materialization, copy-on-write, fallback, or overlay selection;
- state persistence or schema compatibility;
- interruption handling, rollback, or cleanup;
- command execution or user-controlled input;
- shared services used by multiple commands.

High-risk behavior requires focused automated regression coverage and integration testing against disposable repositories whenever technically feasible.

Passing the new test alone is insufficient. Verification MUST also cover the most likely existing behavior that could regress because of the changed code path.

Before completion, explicitly confirm:

- the changed contract is directly tested;
- adjacent behavior identified in the risk assessment is tested or otherwise verified;
- every modified shared abstraction has representative consumer coverage;
- destructive failure paths leave the disposable repository and filesystem in the documented state;
- no existing test was weakened, removed, ignored, or broadened merely to make the change pass.

---

# Shared Abstractions and Change Size

Shared helpers, services, adapters, and cross-command abstractions are stable contracts by default.

Change a shared abstraction only when:

1. the root cause belongs there;
2. all meaningful consumers have been inspected;
3. the broader behavior is intentional;
4. regression risk has been assessed;
5. verification covers representative consumers.

Prefer:

- the narrowest safe scope;
- standard Rust, Cargo, Git, and platform behavior over custom mechanisms;
- existing dependencies over new dependencies;
- local adaptation over expanding shared behavior;
- deletion or simplification over speculative flexibility.

Avoid unrelated refactors and formatting churn.

## Structural Refactoring

When splitting a monolithic production or integration-test file:

1. run and record the complete applicable test baseline before moving code;
2. split by existing reasons to change and side-effect boundaries, not by an arbitrary line target;
3. prefer private modules in the current crate before proposing another crate;
4. preserve public APIs, persisted formats, errors, and observable CLI behavior unless the task explicitly changes them;
5. keep visibility narrow and keep tests with the responsibility or observable scenario they protect;
6. rerun focused tests after each move and the same complete quality gates after restructuring;
7. report the largest-file and total-line effects, acknowledging reasonable module-boundary overhead;
8. update routed architecture documentation when the resulting boundaries are reusable.

The detailed binding workflow is owned by the structural refactoring section of the routed Rust/TDD development standard.

---

# Dependencies and External Knowledge

Before adding or changing a dependency:

1. confirm the need cannot be met safely by the standard library or an existing dependency;
2. inspect the project's package manager, runtime version, and current dependency conventions;
3. verify the exact API and compatibility using official documentation;
4. consider portability, maintenance, installation, security, and native-build implications;
5. keep the dependency scope minimal.

Do not invent package versions, command flags, platform support, or Git behavior from memory when they can be verified.

---

# Test and Verification Rules

Automated tests and TDD are mandatory for every production behavior change. The depth of additional regression coverage must be proportional to behavior and risk.

Automated tests are required for:

- product and business rules;
- CLI behavior and exit codes;
- Git/worktree state transitions;
- filesystem and materialization behavior;
- path, symlink, and input validation;
- configuration or persisted-state behavior;
- reproducible bug fixes;
- recovery, rollback, and failure paths;
- shared abstractions.

Wording-only documentation changes and formatting-only changes do not require artificial production tests. In those cases, verify links, formatting, and consistency directly.

Testing rules:

- Observe the focused test fail before writing production behavior.
- Prefer deterministic tests with isolated temporary directories.
- Use real temporary Git repositories for behavior that mocks cannot prove.
- Cover both success and failure behavior.
- Assert externally observable state, not only internal calls.
- Include interruption or partial-failure coverage for multi-step destructive operations when feasible.
- Never weaken assertions, delete coverage, mark tests ignored, or replace meaningful integration coverage with mocks solely to obtain a passing suite.
- Treat a test that passes before the intended implementation as evidence that the test does not prove the new contract; correct the test before continuing.
- Keep production-only test seams out of public APIs unless a documented design requires them.
- Do not claim a task is complete solely because a broad test command passes; confirm that the changed contract is actually exercised.
- If a relevant check cannot run, report why and perform the strongest safe alternative.

Once a Cargo workspace exists, the default completion gates are:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Run platform-specific tests and `--all-features` variants when the changed behavior or workspace configuration requires them. Do not claim cross-platform support from tests run on only one platform; distinguish direct evidence from unverified compatibility.

---

# Documentation Maintenance

Perform a documentation impact check for every non-trivial change.

Update documentation when a change introduces or alters:

- product behavior or scope;
- an invariant or decision rule;
- CLI, configuration, or state contracts;
- architecture or component responsibilities;
- security, recovery, Git, or filesystem behavior;
- reusable development or verification conventions.

Update the owning leaf document, its router, and every affected ancestor router in the same task.

If no documentation update is needed, state why in the final report.

---

# Definition of Done

A task is not complete when any applicable item is missing:

- routed documentation was consulted;
- acceptance criteria were satisfied;
- regression risk was assessed;
- a failing test was observed before production behavior was implemented;
- the Red-Green-Refactor cycle was followed;
- relevant success, failure, and recovery behavior was verified;
- automated tests protect the changed contract and likely regressions;
- existing behavior and compatibility were preserved;
- Rust formatting, compilation, lint, and test gates passed;
- documentation impact was assessed and routed docs were updated when needed;
- changed documentation links and router topology were verified;
- limitations, assumptions, and unverified areas were reported.

---

# Minimum Final Checklist

- [ ] Applicable documentation routes followed
- [ ] Repository conventions and current implementation inspected
- [ ] Acceptance criteria and regression surfaces identified
- [ ] Failing test observed before production implementation
- [ ] Red-Green-Refactor cycle completed
- [ ] Minimal scoped implementation completed
- [ ] Changed behavior and likely regressions are directly protected
- [ ] Focused tests and complete Cargo quality gates passed
- [ ] Destructive behavior tested only in disposable environments
- [ ] Documentation impact assessed
- [ ] Routers updated for documentation topology changes
- [ ] Commit convention followed when a commit was created or proposed
- [ ] Remaining assumptions or limitations reported

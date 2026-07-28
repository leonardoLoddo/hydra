# Rust, TDD, and Anti-Regression Standard

## Purpose

This document defines the binding engineering workflow for Hydra.

Hydra is implemented in Rust. Every production behavior change is developed test-first, and every change must protect both the intended contract and the existing behavior most likely to regress.

---

## Rust Baseline

- Production application code and core libraries are written in Rust.
- Cargo is the build, dependency, test, and workspace system.
- The repository-declared Rust toolchain and edition are authoritative.
- Safe Rust is the default.
- Platform-specific behavior is isolated behind narrow interfaces so portable logic can be tested independently.
- Another runtime or language must not become a dependency of core Hydra behavior without an explicit architectural decision.

`unsafe` code is allowed only when all of the following are true:

1. a required platform capability cannot be implemented adequately with safe Rust;
2. the unsafe block is isolated behind the smallest practical safe interface;
3. its safety invariants are documented next to the boundary;
4. tests exercise valid use, invalid inputs, and relevant failure behavior;
5. the change is treated as high regression risk.

Expected failures from user input, Git, filesystem state, configuration, or platform capabilities return contextual errors rather than panicking.

---

## Mandatory TDD Cycle

Every change that can affect compiled behavior, runtime behavior, CLI output, exit status, persisted state, error behavior, or platform behavior follows this cycle:

1. **Specify** one observable requirement and its acceptance criteria.
2. **Assess regression risk** and identify adjacent behavior that shares the changed path.
3. **Red**: write or update the smallest automated test that proves the requirement.
4. Run the focused test and observe it fail for the expected reason.
5. **Green**: implement only enough production code to make the test pass.
6. Run the focused test and observe it pass.
7. **Refactor** only with the relevant tests green.
8. Repeat for the next behavior.
9. Run the broader regression suite and Rust quality gates.

An implementation followed by tests is not TDD.

A test that already passes before the behavior is implemented does not establish a Red phase. Improve the test until it fails for the missing contract.

A compile failure may count as Red when the new test intentionally references a missing API or variant and the diagnostic demonstrates that exact missing contract. Unrelated compilation failures do not count.

---

## Task-Specific TDD Rules

### Features

- Derive tests from acceptance criteria, not from the planned implementation.
- Add one behavior at a time.
- Cover success, expected rejection, and state left after failure.
- Do not add speculative behavior merely to complete an abstraction.

### Bug Fixes

- Reproduce the reported defect in a failing regression test before changing production code.
- Confirm the test fails because of the defect, not because of unrelated setup.
- After the fix, retain the test permanently.
- Verify the nearest related behavior that could be affected by the same change.

### Refactors

- Establish behavioral coverage before restructuring.
- When existing tests do not protect the behavior, add characterization tests first.
- Keep observable behavior unchanged unless the task explicitly requires otherwise.
- Run tests for representative consumers of every changed shared abstraction.

### Dependency and Toolchain Changes

- Define the behavior or compatibility need before changing the dependency.
- Run the existing suite before and after the change.
- Verify supported targets and feature combinations affected by the dependency.
- Do not combine dependency upgrades with unrelated behavior changes.

---

## Test Layers

Use the lowest layer that proves the contract, then add higher-level coverage when integration is itself part of the risk.

### Unit Tests

Use unit tests for deterministic parsing, validation, state transitions, selection rules, path-independent logic, and error classification.

### Integration Tests

Use integration tests with newly created temporary repositories and directories for:

- Git refs and worktrees;
- index and working-tree isolation;
- Head lifecycle operations;
- state persistence and reconciliation;
- overlay selection and materialization;
- filesystem failure and rollback behavior;
- command execution and exit status.

Mocks cannot prove Git or filesystem integration contracts. Use real disposable resources when those contracts matter.

### CLI Tests

CLI tests verify observable behavior:

- exit status;
- stdout and stderr ownership;
- human-readable and JSON output contracts;
- absence of unintended side effects;
- state left after both success and failure.

Avoid brittle assertions on wording when the contract is semantic. Assert exact text when output is a documented interface.

### Platform-Specific Tests

- Keep portable contract tests separate from native backend tests.
- Detect capabilities on the actual test volume.
- A skipped native-backend test must state the missing capability.
- Always test the safe copy fallback where applicable.
- Results from one operating system are not evidence that another operating system passed.

---

## Disposable Test Environment

Tests that mutate Git or the filesystem MUST:

- create a unique temporary root;
- initialize their own repository and configuration;
- avoid the Hydra source repository and all existing user repositories or Heads;
- avoid ambient global Git configuration when it could affect determinism;
- use explicit paths and validated fixtures;
- clean up when safe while preserving enough diagnostics on failure where the harness supports it;
- remain safe under parallel execution.

Destructive targets must never be derived from a broad directory, unresolved glob, or unvalidated environment variable.

---

## Anti-Regression Contract

Before production code changes, identify:

- the exact existing contracts on the changed path;
- direct and indirect consumers of modified shared code;
- data, refs, paths, state, and error flows that may be affected;
- destructive or difficult-to-recover failure modes;
- the tests required for the new behavior and likely regressions.

Every change must protect:

1. the intended new or corrected behavior;
2. the most likely adjacent existing behavior to regress;
3. representative consumers when shared code changes;
4. failure-state integrity for destructive or multi-step operations.

It is forbidden to obtain a green suite by:

- weakening an assertion without a contract change;
- deleting a relevant test;
- marking a relevant test ignored;
- changing a fixture so it no longer exercises the defect;
- replacing meaningful integration coverage with a mock;
- broadening accepted behavior without an explicit requirement;
- retrying or swallowing failures that should remain visible.

A passing suite is necessary but not sufficient. The changed and adjacent contracts must be directly exercised.

---

## Rust Testability Rules

- Prefer dependency injection through small traits or explicit function parameters only where a real boundary exists.
- Keep pure decision logic separate from side effects when this improves direct testing.
- Do not expose production internals publicly only to make them testable.
- Do not add runtime branches that exist solely for tests.
- Use deterministic clocks, identifiers, and failure injection at explicit boundaries when the behavior depends on them.
- Avoid assertions tied to internal call order unless ordering is itself a contract.
- Make concurrent tests independent and free from shared mutable global state.

---

## Completion Gates

Run the narrow Red and Green commands during development and retain their outcome as TDD evidence in the final report.

Once the Cargo workspace exists, every production code change must pass:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Also run:

- tests for platform-specific backends affected by the change;
- `--all-features` checks when the workspace's feature combinations support it;
- targeted integration tests for changed Git or filesystem behavior;
- release-mode or performance checks when the task changes correctness assumptions that differ under optimization or has an explicit performance contract.

If a required gate cannot run, the task is not silently complete. Report the exact limitation and the strongest safe evidence available.

---

## Required Final Evidence

For production changes, the final report states:

- the Red test and why it failed;
- the minimal behavior added during Green;
- any refactor performed after Green;
- focused and full-suite commands run;
- regression paths protected;
- platform or feature combinations not directly verified;
- documentation impact.

Pure documentation changes report document/link validation instead of artificial Rust test evidence.

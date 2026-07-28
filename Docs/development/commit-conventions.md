# Commit Conventions

## Purpose

Hydra uses Conventional Commits to keep its history readable and suitable for long-term maintenance, automated changelogs, semantic versioning, release tooling, and targeted history searches.

This document is binding for commit messages authored in the Hydra repository.

---

## Message Format

The first line uses:

```text
<type>[optional scope][optional !]: <description>
```

Examples:

```text
feat(cli): add head create command
fix(git): resolve detached HEAD detection
docs: update MVP specification
refactor(storage): simplify materializer selection
```

The scope is optional. The `!` marks a breaking change.

A longer commit may add a body and footers:

```text
<type>[optional scope][optional !]: <description>

[optional body]

[optional footer(s)]
```

---

## Allowed Types

Use the narrowest type that describes the logical change.

| Type | Use when | Example |
|---|---|---|
| `feat` | Introducing externally observable functionality or capability | `feat(cli): implement init command` |
| `fix` | Correcting a defect or restoring an existing contract | `fix(config): handle missing hydra.json` |
| `refactor` | Restructuring production code without changing observable behavior | `refactor(core): split project discovery` |
| `docs` | Changing documentation only | `docs: update architecture` |
| `test` | Changing tests or test infrastructure without changing production behavior | `test(git): add worktree discovery coverage` |
| `chore` | Maintaining the repository without changing product behavior, build output, CI, or tests | `chore: initialize repository metadata` |
| `perf` | Improving measurable performance without changing the intended behavior | `perf(storage): reduce filesystem scans` |
| `ci` | Changing CI workflows or CI-specific configuration | `ci: build release binaries` |
| `build` | Changing the build system, packaging, compilation targets, or build dependencies | `build: add Windows target` |

Type selection rules:

- Tests and documentation required by a feature or fix belong in the same `feat` or `fix` commit. Do not split them into separate `test` or `docs` commits merely because they affect different file kinds.
- Use `test` only when the logical change is exclusively test coverage, fixtures, test helpers, or test infrastructure.
- Use `docs` only when the logical change is exclusively documentation.
- Use `ci` for automation executed by the CI platform and `build` for artifacts, compilation, packaging, or build dependencies.
- `chore` is not a fallback for a message that has not been classified. Prefer a more precise type whenever one applies.
- A `perf` commit must be supported by relevant measurement or a clearly defined performance contract.

Do not invent a new type without updating this document and its owning router.

---

## Scopes

Use a scope only when it adds useful ownership or subsystem information.

Preferred Hydra scopes are:

```text
cli
core
git
config
storage
materializer
overlay
doctor
head
session
ci
docs
```

Examples:

```text
feat(head): implement create command
fix(storage): detect unsupported CoW
test(git): cover detached worktree discovery
```

Scope rules:

- Write scopes in lowercase.
- Use one scope that represents the primary affected subsystem.
- Omit the scope for repository-wide changes or when it would repeat the type without adding information.
- Do not list multiple scopes in one header.
- Introduce a new scope only for a stable project area that will remain useful in history searches. Update this document when the preferred scope vocabulary changes.

---

## Descriptions

The description MUST:

- be written in English;
- begin with a lowercase verb;
- use the imperative mood;
- describe the completed logical change;
- remain concise and specific;
- omit the final period.

Correct:

```text
feat(cli): add init command
fix(storage): preserve fallback after clone failure
```

Incorrect:

```text
Added init command
Adds init command.
update
changes
stuff
wip
prova
test
```

Avoid vague verbs such as `update`, `change`, or `improve` unless the object and effect make the message unambiguous.

---

## Body

Use a body when the reason, constraints, or non-obvious consequences do not fit in the header.

The body should explain:

- why the change is necessary;
- which contract or constraint shaped the solution;
- important consequences or limitations;
- context that will still matter during future maintenance.

Do not turn the body into a file-by-file change log. The diff already records implementation details.

Write the body in English and separate it from the header with one blank line.

---

## Breaking Changes

Mark a compatibility-breaking change with `!` before the colon:

```text
feat!: redesign head storage
feat(storage)!: redesign metadata layout
```

Add a `BREAKING CHANGE:` footer that states what became incompatible and what users or integrators must do:

```text
feat(storage)!: redesign metadata layout

BREAKING CHANGE: Existing version 1 state files must be migrated before Hydra can open them.
```

A breaking change requires:

- an intentional compatibility decision;
- explicit migration or recovery guidance;
- appropriate tests;
- updated routed documentation;
- release impact assessment.

Do not use `!` for an internal refactor with no externally observable compatibility impact.

---

## Footers

Footers follow the body after one blank line.

Use them for structured metadata such as:

```text
BREAKING CHANGE: Existing configuration requires migration.
Refs: #123
Closes: #456
```

Do not put issue identifiers or breaking-change details into an otherwise vague description.

---

## Atomic Commit Rules

One commit represents one coherent logical change.

Each commit should:

- have a single reviewable purpose;
- include the production code, TDD tests, and documentation required to make that purpose complete;
- preserve repository consistency and pass the applicable quality gates;
- exclude unrelated formatting, refactors, generated files, or cleanup;
- be understandable without relying on later commits to explain its intent.

Separate changes when they can be reviewed and reverted independently. Keep them together when separating them would leave an incomplete behavior, missing regression protection, or outdated documentation.

TDD describes the order of work in the working tree; it does not require separate Red, Green, and Refactor commits.

---

## History Integrity

- Do not create vague commits such as `fix`, `update`, `changes`, `stuff`, `wip`, `prova`, or `test`.
- Do not place WIP commits on the main branch or in history intended for release.
- Do not mix unrelated changes to reduce the number of commits.
- Do not rewrite, amend, squash, or force-push shared history without explicit authorization.
- Do not bypass repository hooks or required checks to create a commit.
- Review the staged diff before committing and ensure the message describes exactly that diff.

Temporary local commits may be reorganized before integration, but the resulting shared history MUST follow this document.

---

## Examples

```text
chore: initialize Rust workspace
docs: add project README
feat(cli): add command parser
feat(cli): implement init command
feat(config): create hydra.json
feat(core): detect Git repository
feat(storage): create heads directory
test(core): add repository discovery edge cases
docs: document storage layout
```

A feature developed with TDD normally lands as one complete commit:

```text
feat(head): implement head creation
```

That commit includes the production implementation, the tests written during Red-Green-Refactor, and any documentation required by the new behavior.

---

## Commit Checklist

Before creating or approving a commit:

- [ ] The commit contains one logical change
- [ ] The type describes that logical change
- [ ] The optional scope adds useful information
- [ ] The description is English, imperative, lowercase, specific, and has no final period
- [ ] Feature or fix tests are included in the same commit
- [ ] Required documentation is included
- [ ] Breaking changes use `!` and a `BREAKING CHANGE:` footer
- [ ] Applicable Rust and regression checks pass
- [ ] The staged diff contains no unrelated changes

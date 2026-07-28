# Development Context Router

## Purpose

This router owns the reusable engineering conventions used to implement and maintain Hydra: language and toolchain constraints, test-driven development, test strategy, Rust quality gates, anti-regression requirements, and commit history conventions.

It does not own product behavior, CLI contracts, Git semantics, storage design, or filesystem safety guarantees. Those belong to their respective routed domains.

---

## Routes

| Document | Consult when the task involves | Nature of the document |
|---|---|---|
| [`rust-tdd-context.md`](rust-tdd-context.md) | Any Rust code, test, bug fix, refactor, dependency, Cargo configuration, quality gate, platform-specific implementation, unsafe boundary, or regression assessment | Binding Rust engineering and test-driven development standard. It defines the Red-Green-Refactor workflow, test layers, anti-regression duties, and completion gates. |
| [`commit-conventions.md`](commit-conventions.md) | Creating, amending, squashing, reviewing, proposing, or documenting commits; choosing commit types or scopes; describing breaking changes; preparing history for releases or changelogs | Binding Conventional Commits standard for Hydra. It defines the allowed message format, types, scopes, descriptions, bodies, footers, atomicity rules, and examples. |

---

## Consultation Rules

- Consult `rust-tdd-context.md` before every change that can affect compiled or runtime behavior.
- Consult it for code review even when no implementation is requested, because regressions and missing TDD evidence are review concerns.
- Consult `commit-conventions.md` before creating or changing a commit and when reviewing commit-message or history quality.
- Combine this route with the product route whenever behavior visible to Hydra users may change.
- Combine it with future technical routers for CLI, Git, storage, configuration, security, or architecture when those domains are affected.

Routes are cumulative. Development rules do not replace the behavior and safety contracts owned by other domains.

---

## Domain Maintenance

Add a development document only when it owns a stable engineering concern that cannot remain clear in the existing Rust/TDD standard.

When adding, renaming, moving, or removing a document:

1. update the **Routes** table;
2. update consultation rules when ownership changes;
3. update the parent router at [`../hydra-context-router.md`](../hydra-context-router.md) if the domain description or path changes;
4. verify all relative links.

Do not store task plans, test run logs, benchmark snapshots, or temporary investigation notes in this folder.

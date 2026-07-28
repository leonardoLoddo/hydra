# Product Context Router

## Purpose

This router owns Hydra's product definition: the problem being solved, product principles, MVP behavior, user-visible contracts, scope boundaries, and success criteria.

It does not own implementation-level conventions once those are documented in dedicated architecture, Git, storage, CLI, security, or testing domains.

---

## Routes

| Document | Consult when the task involves | Nature of the document |
|---|---|---|
| [`hydra-mvp-context.md`](hydra-mvp-context.md) | Product direction, Head semantics or lifecycle, Git/worktree behavior, materialization and overlay expectations, configuration, CLI scope, safety requirements, supported platforms, MVP acceptance criteria, or roadmap boundaries | Foundational product and MVP specification. It defines what Hydra is, what v0.1 must do, the currently recommended technical direction, and what remains out of scope. |

---

## Consultation Rules

- Consult `hydra-mvp-context.md` for every feature, bug fix, architectural decision, or documentation change that could alter externally observable Hydra behavior.
- Consult it before adding functionality to confirm whether the behavior belongs in the MVP.
- Treat its safety guarantees and Definition of Done as binding product requirements.
- If a task affects only internal implementation details without changing product behavior, consult the future owning technical router as well; when no such router exists yet, use the MVP document for constraints and inspect the codebase for established conventions.
- If a future focused product document overlaps this context, consult both until the foundational document explicitly delegates ownership.

Routes are cumulative. Never stop after the first matching document when multiple rows apply.

---

## Domain Maintenance

Add a product document only when it contains reusable product knowledge with a distinct responsibility that would make the foundational context materially clearer.

When adding, renaming, moving, or removing a document:

1. update the **Routes** table;
2. update the consultation rules if ownership changes;
3. update the parent router at [`../hydra-context-router.md`](../hydra-context-router.md) if the domain description or path changes;
4. verify all relative links.

Do not use this folder for task notes, changelogs, implementation diaries, or temporary plans.

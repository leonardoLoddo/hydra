# Hydra Project Knowledge Router

**LibrAIrian Protocol:** 1.0.0

## Purpose

This is the single entry point for Hydra's repository-local, AI-facing project
knowledge. Human-facing documentation under `Docs/user/` is outside the active
knowledge graph and is loaded only when a task explicitly requires user
documentation maintenance or verification.

## Routing algorithm

1. Read this router before classifying the task against repository concerns.
2. Identify every concern touched by the task.
3. Follow every matching route. Routes are cumulative.
4. Process each router once.
5. Read required knowledge before recommended or reference material.
6. Resolve explicit inherited-default dependencies of selected leaves.
7. Combine selected knowledge with targeted code, tests, configuration,
   version-control state, and runtime evidence.
8. Report material conflicts or unresolved ambiguity.

## Controlled concern vocabulary

| Concern | Meaning | Excludes |
|---|---|---|
| Product | Product identity, supported scope, Head semantics, configuration and overlays, storage and platforms, lifecycle, state and recovery, CLI interaction, and separate roadmap proposals | Implementation structure and engineering workflow |
| Architecture | Implemented crate boundaries, independently selected materialization mechanics, Git and filesystem workflows, persistence, rollback, inspection, repair, and command internals | Product policy not yet represented in implementation |
| Development | Rust and TDD rules, commit conventions, releases, distribution, and the installable Hydra Agent Skill | Product and architecture contracts owned by their domains |
| Governance | LibrAIrian adoption, routing, authoring, ownership, audits, upgrades, and knowledge-impact maintenance | Initial implementation investigation before knowledge maintenance or completion |

## Routes

| Conditions | Priority | Read | Skip when |
|---|---|---|---|
| any: product identity, Head lifecycle or semantics, configuration or overlay policy, storage or platform, state or recovery, CLI interaction, compatibility, safety guarantee, roadmap boundary | required | [product/ROUTER.md](product/ROUTER.md) | the task is purely internal and cannot affect product behavior or scope |
| any: crate boundary, Git or filesystem workflow, initialization, Head command internals, storage, persistence, rollback, inspection, or repair | required | [architecture/ROUTER.md](architecture/ROUTER.md) | the task changes only product policy or engineering process |
| any: implementation task, repository change, engineering workflow, Rust code or test, bug fix, refactor, dependency, quality gate, commit or history operation, release or distribution, Hydra Agent Skill | required | [development/ROUTER.md](development/ROUTER.md) | read-only consultation with no repository change or engineering decision |
| any: implementation completion, protocol adoption, router or leaf change, knowledge authoring or review, audit, upgrade, fallback operation, knowledge-impact check | required | [governance/ROUTER.md](governance/ROUTER.md) | initial implementation investigation before knowledge maintenance or completion |

## Cross-context composition

- A user-visible implementation change selects Product, Architecture, and
  Development.
- A release that changes supported artifacts selects Product and Development.
- A knowledge change selects Governance and every domain that owns affected
  knowledge.
- A human documentation task does not select `Docs/user/` through this graph.
  Follow the repository's explicit user-documentation rules only when that
  task or an implementation change requires it.

## No exact match

Use the closest route for investigation. Do not create current knowledge until
scope, evidence, and canonical ownership are established. If ambiguity would
materially change product behavior, architecture, safety, compatibility, or
scope, request an explicit decision.

## Ownership and maintenance

- Routers select; leaves explain.
- Every leaf has one canonical owning router.
- Cross-context routes link without copying authority.
- Update affected owning and ancestor routers when an artifact is created,
  moved, renamed, split, merged, demoted, or removed.
- Keep human-facing documentation outside the active knowledge graph.

## Verification

Follow the Governance validation route for structural checks, routing cases,
semantic evidence, inheritance, and skill-unavailable fallback verification.
Read-only consultation does not require an audit of unrelated knowledge.

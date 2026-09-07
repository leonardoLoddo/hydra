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
6. Combine selected knowledge with targeted code, tests, configuration,
   version-control state, and runtime evidence.
7. Report material conflicts or unresolved ambiguity.

## Controlled concern vocabulary

| Concern | Meaning | Excludes |
|---|---|---|
| Product | Product identity, supported scope, Head semantics, user-visible contracts, safety guarantees, and roadmap boundaries | Implementation structure and engineering workflow |
| Architecture | Implemented crate boundaries, Git and filesystem workflows, persistence, rollback, inspection, repair, and command internals | Product policy not yet represented in implementation |
| Development | Rust and TDD rules, commit conventions, releases, distribution, and the installable Hydra Agent Skill | Product and architecture contracts owned by their domains |
| Governance | LibrAIrian adoption, routing, authoring, ownership, audits, upgrades, and knowledge-impact maintenance | Ordinary implementation after all affected project routes are selected |

## Routes

| Conditions | Priority | Read | Skip when |
|---|---|---|---|
| any: product identity, Head lifecycle or semantics, supported scope or platform, user-visible behavior, safety guarantee, roadmap boundary | required | [product/ROUTER.md](product/ROUTER.md) | the task is purely internal and cannot affect product behavior or scope |
| any: crate boundary, Git or filesystem workflow, initialization, Head command internals, storage, persistence, rollback, inspection, or repair | required | [architecture/ROUTER.md](architecture/ROUTER.md) | the task changes only product policy or engineering process |
| any: Rust code or test, bug fix, refactor, dependency, quality gate, commit or history operation, release or distribution, Hydra Agent Skill | required | [development/ROUTER.md](development/ROUTER.md) | the task changes only product policy or human documentation |
| any: protocol adoption, router or leaf change, knowledge authoring or review, audit, upgrade, fallback operation, knowledge-impact check | required | [governance/ROUTER.md](governance/ROUTER.md) | ordinary implementation after every affected knowledge route is selected |

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

Validate `.agents/skills/librairian/` and `skills/hydra/` with the Agent Skill
validator. Run concrete Markdown link checks, YAML parsing, and `git diff
--check`. Exercise at least one positive, near-miss, cumulative, and
knowledge-maintenance routing case for routing changes.

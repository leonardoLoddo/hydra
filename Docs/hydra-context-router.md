# Hydra Documentation Context Router

## Purpose

This file is the mandatory entry point for Hydra documentation.
It routes AI agents and contributors to the relevant documentation domains without duplicating the content owned by those domains.

Before proposing or implementing a change, consult every applicable route. Routes are cumulative: a task that touches more than one domain requires following each relevant router through to its leaf documents.

---

## How to Use This Router

1. Classify the task by the domains it affects.
2. Open every matching domain router listed below.
3. Follow the consultation rules in those routers until the required leaf documents have been read.
4. If no route covers the task, inspect the existing project and identify the missing documentation domain before making a material architectural assumption.
5. If documentation conflicts with code or observed behavior, stop and report the conflict explicitly.

Reading this file alone is never sufficient when a matching child router exists.

---

## Documentation Routes

| Domain | Consult when the task involves | Router |
|---|---|---|
| Product | Product definition, MVP scope, Head lifecycle, user-facing behavior, constraints, supported platforms, or roadmap boundaries | [`product/product-context-router.md`](product/product-context-router.md) |
| User | Italian end-user workflows, command usage, configuration guidance, customization, troubleshooting, or the implemented-versus-planned boundary | [`user/user-context-router.md`](user/user-context-router.md) |
| Architecture | Implemented component boundaries, crate responsibilities, dependency direction, project initialization internals, persistence workflows, or rollback design | [`architecture/architecture-context-router.md`](architecture/architecture-context-router.md) |
| Development | Any Rust code, tests, bug fix, refactor, dependency, Cargo configuration, quality gate, regression assessment, commit, or history convention | [`development/development-context-router.md`](development/development-context-router.md) |

---

## Cross-Domain Routing

Routes are cumulative rather than exclusive.

For example, a future change to Head creation may require consulting:

- the product router for the intended behavior and MVP boundary;
- an architecture router for component responsibilities;
- a Git or storage router for implementation invariants;
- a testing router for required verification.

Add other domain routes when their first reusable document is introduced; do not create speculative empty domains.

---

## Router Contract

Every direct subfolder of `Docs/` that contains project documentation MUST contain one context router.

Router filenames use this convention:

```text
Docs/<domain>/<domain>-context-router.md
```

Each domain router MUST:

- state the domain it owns and its boundaries;
- describe every direct child document and nested router;
- define when each route must be consulted;
- link to documents using relative paths;
- route cumulatively when multiple documents apply;
- avoid duplicating rules owned by leaf documents.

Leaf documents contain the actual source-of-truth rules, decisions, contracts, and rationale. Routers contain navigation metadata only.

---

## Documentation Topology Maintenance

When documentation is added, renamed, moved, or removed:

1. update the router in the document's owning folder;
2. update each affected ancestor router up to this file;
3. verify every changed relative link;
4. remove stale routes;
5. keep the route description precise enough that a future agent knows when consultation is mandatory.

When a new domain is created:

1. create `Docs/<domain>/`;
2. create `Docs/<domain>/<domain>-context-router.md`;
3. register that router in the **Documentation Routes** table above;
4. add the domain's first leaf document in the same change.

Do not add individual leaf-document routes to `AGENTS.md`. `AGENTS.md` links only to this macro-router.

---

## Current Documentation Map

```text
Docs/
├── hydra-context-router.md
├── architecture/
│   ├── architecture-context-router.md
│   ├── head-close.md
│   ├── head-creation.md
│   ├── head-inspection.md
│   ├── head-open.md
│   ├── head-removal.md
│   ├── project-initialization.md
│   ├── repair.md
│   └── system-architecture.md
├── development/
│   ├── commit-conventions.md
│   ├── development-context-router.md
│   └── rust-tdd-context.md
├── product/
│   ├── product-context-router.md
│   └── hydra-mvp-context.md
└── user/
    ├── user-context-router.md
    └── hydra-user-guide.it.md
```

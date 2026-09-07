# Architecture Project Knowledge Router

## Purpose and scope

This router owns implemented crate boundaries and the Git, filesystem,
persistence, rollback, inspection, repair, and command workflows that realize
Hydra's product contract. It excludes unimplemented product policy and generic
Rust engineering rules.

## Selection rules

| Conditions | Priority | Read | Skip when |
|---|---|---|---|
| any: crate boundary, dependency direction, CLI versus core responsibility | required | [system-architecture.md](system-architecture.md) | no structural responsibility changes or decisions are involved |
| any: `hydra init`, repository discovery, initial configuration or state, Heads directory, initialization rollback | required | [project-initialization.md](project-initialization.md) | initialization cannot be affected |
| any: `hydra head create`, Head name, base or target ref, private branch, worktree registration, overlay materialization, creation lock, rollback or cleanup | required | [head-creation.md](head-creation.md) | Head creation cannot be affected |
| any: `hydra status`, `head list`, `head status`, `head path`, inventory reads, ahead or behind comparison, read-only consistency reporting | required | [head-inspection.md](head-inspection.md) | no inspection behavior or shared inventory read is involved |
| any: `hydra head remove`, forced removal, worktree deletion, inventory removal, private-branch preservation, partial-removal recovery | required | [head-removal.md](head-removal.md) | removal and its shared protected workflow cannot be affected |
| any: `hydra head close`, target integration, native Git merge, conflict continuation or abort, close adapter, removal after integration | required | [head-close.md](head-close.md) | close and integration cannot be affected |
| any: `hydra head open`, configured opener, process arguments, placeholders, launch validation or failure | required | [head-open.md](head-open.md) | no configured process launch is involved |
| any: `hydra repair`, inventory reconciliation, recovery record, abandoned lock, missing or moved worktree, guided repair | required | [repair.md](repair.md) | no inconsistency diagnosis or mutation is involved |
| any: `hydra doctor storage`, storage probe, native clone, full-copy fallback, isolation report, diagnostic cleanup | required | [doctor-storage.md](doctor-storage.md) | storage capability and reporting cannot be affected |
| any: `hydra completions`, shell registration, dynamic Head candidate, completion failure, packaged completion | required | [shell-completions.md](shell-completions.md) | shell completion behavior cannot be affected |

## Cross-context composition

- Add [../product/ROUTER.md](../product/ROUTER.md) for user-visible behavior or safety guarantees.
- Add [../development/ROUTER.md](../development/ROUTER.md) for every Rust implementation, test, dependency, quality-gate, commit, or release concern.
- Select every matching architecture leaf. Command workflows may share inventory, configuration, storage, or protected-removal boundaries.

## Routing examples

- Positive: changing repair adoption selects `repair.md` because it mutates reconciliation behavior.
- Negative: editing release prose skips `repair.md` when repair behavior and guidance are unchanged.
- Cumulative: changing removal used by close selects both `head-removal.md` and `head-close.md`.

## Ownership and maintenance

This router canonically owns every leaf in this directory. Update this router
and [../ROUTER.md](../ROUTER.md) when owned knowledge is created, moved,
renamed, split, merged, demoted, or removed.

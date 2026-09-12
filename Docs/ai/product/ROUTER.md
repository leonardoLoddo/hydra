# Product Project Knowledge Router

## Purpose and scope

Select product identity, safety, current behavior, compatibility, and explicitly
proposed scope. Implementation transactions belong to Architecture; engineering
and distribution procedures belong to Development.

## Selection rules

| Conditions | Priority | Read | Skip when |
|---|---|---|---|
| any: product identity, isolation, Git compatibility, core acceptance, SemVer baseline, maturity, inherited Head safety default | required | [hydra-mvp-context.md](hydra-mvp-context.md) | no product invariant or dependent product leaf is selected |
| any: .hydra.json, directory strategy, branch prefix, overlay selection, rule expansion, unsafe-symlink exclusions, storage policy | required | [configuration-and-overlays.md](configuration-and-overlays.md) | configuration and overlay selection cannot be affected |
| any: materialization, CoW, content-source reuse, storage probe, full-copy fallback, performance, platform, symlink, submodule | required | [storage-and-platforms.md](storage-and-platforms.md) | physical storage, isolation, and platform boundaries cannot be affected |
| any: init, Head creation or inspection, base or target semantics, parent context, open or close adapter, integration, removal | required | [lifecycle.md](lifecycle.md) | no lifecycle behavior or configured process contract is involved |
| any: state, schema compatibility, ownership, inventory, lock, recovery record, interruption, repair | required | [state-and-recovery.md](state-and-recovery.md) | persisted intent, ownership, and recovery cannot be affected |
| any: command hierarchy, help, prompt, terminal rendering, exit status, machine output, completion | required | [cli-contract.md](cli-contract.md) | only internal implementation changes with unchanged interaction contracts |
| any: explicit future design, roadmap, scope expansion, proposed capability | reference | [roadmap.md](roadmap.md) | implementing or documenting existing behavior without a roadmap question |

## Cross-context composition

- Resolve the explicit inherited defaults of each selected leaf once.
- Add [../architecture/ROUTER.md](../architecture/ROUTER.md) when inspecting or
  changing implementation workflows, side effects, or component boundaries.
- Add [../development/ROUTER.md](../development/ROUTER.md) for implementation,
  verification, commits, releases, and Hydra skill maintenance.
- Add [../governance/ROUTER.md](../governance/ROUTER.md) for knowledge changes.

## Routing examples

- Positive: a storage probe change selects storage, lifecycle only if init or
  create behavior changes, and the inherited core safety contract.
- Near-miss: source formatting does not select roadmap or CLI interaction.
- Cumulative: persistent unsafe-symlink exclusion selects configuration,
  storage, lifecycle, state, CLI, and their inherited safety default.
- Maintenance: translating a product leaf selects Governance and its owning
  product contract; translation does not authorize implementing proposals.

## Ownership and maintenance

This router canonically owns all seven leaves registered above. Other domains
may depend on them without taking ownership. Update this router and
[../ROUTER.md](../ROUTER.md) when ownership, paths, or selection boundaries change.

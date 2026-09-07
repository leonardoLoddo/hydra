# Product Project Knowledge Router

## Purpose and scope

This router owns Hydra's product definition, supported scope, Head semantics,
user-visible contracts, safety guarantees, and roadmap boundaries. It excludes
implementation structure and engineering workflow.

## Selection rules

| Conditions | Priority | Read | Skip when |
|---|---|---|---|
| any: product direction, Head semantics or lifecycle, Git or worktree behavior, materialization or overlay expectations, configuration, CLI scope, safety requirement, supported platform, acceptance criterion, roadmap boundary | required | [hydra-mvp-context.md](hydra-mvp-context.md) | the task is purely internal and cannot affect product meaning or externally observable behavior |

## Cross-context composition

- Add [../architecture/ROUTER.md](../architecture/ROUTER.md) when the task affects implemented component boundaries or command behavior.
- Add [../development/ROUTER.md](../development/ROUTER.md) for implementation, tests, dependencies, commits, releases, or Agent Skill maintenance.

## Routing examples

- Positive: changing Head isolation selects `hydra-mvp-context.md` because it affects a core safety guarantee.
- Negative: reformatting Rust without behavior changes skips this route because product meaning is unchanged.
- Cumulative: changing `head create` selects Product, Architecture, and Development because it changes a user-visible contract, implementation workflow, and tested Rust behavior.

## Ownership and maintenance

This router canonically owns `hydra-mvp-context.md`. Update this router and
[../ROUTER.md](../ROUTER.md) when owned knowledge is created, moved, renamed,
split, merged, demoted, or removed.

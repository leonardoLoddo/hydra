# Development Project Knowledge Router

## Purpose and scope

This router owns Hydra's reusable Rust engineering, testing, commit, release,
distribution, and Agent Skill maintenance rules. It excludes product behavior
and implementation architecture owned by other domains.

## Selection rules

| Conditions | Priority | Read | Skip when |
|---|---|---|---|
| any: Rust code or test, bug fix, refactor, dependency, Cargo configuration, quality gate, platform-specific implementation, unsafe boundary, regression assessment | required | [rust-tdd-context.md](rust-tdd-context.md) | the change is documentation-only and cannot affect compiled or runtime behavior |
| any: create, amend, squash, review, propose, or document a commit; choose type or scope; prepare release history | required | [commit-conventions.md](commit-conventions.md) | no commit or history operation is involved |
| any: create, change, package, document, validate, or assess impact on Hydra's installable Agent Skill; change agent-operable workflow or safety guidance | required | [hydra-skill-context.md](hydra-skill-context.md) | the task cannot change what an agent executes, decides, validates, or reports |
| any: repository naming, version, release automation, GitHub Release, packaged artifact, Homebrew, onboarding, release-time skill packaging | required | [release-distribution.md](release-distribution.md) | release and distribution contracts cannot be affected |

## Cross-context composition

- Add [../product/ROUTER.md](../product/ROUTER.md) when behavior visible to users or supported scope may change.
- Add [../architecture/ROUTER.md](../architecture/ROUTER.md) when crate boundaries, Git or filesystem workflows, persistence, initialization, or rollback may change.

## Routing examples

- Positive: changing a Rust dependency selects `rust-tdd-context.md`.
- Negative: correcting human-guide punctuation skips `rust-tdd-context.md` when behavior and technical contracts are unchanged.
- Cumulative: releasing a command change selects `rust-tdd-context.md`, `release-distribution.md`, and the applicable Product and Architecture routes.

## Ownership and maintenance

This router canonically owns `rust-tdd-context.md`, `commit-conventions.md`,
`hydra-skill-context.md`, and `release-distribution.md`. Update this router and
[../ROUTER.md](../ROUTER.md) when owned knowledge is created, moved, renamed,
split, merged, demoted, or removed.

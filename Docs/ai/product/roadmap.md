# Future Product Direction

**Status:** proposed
**Scope:** unimplemented capabilities and product hypotheses; no current command authorization
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read only for explicit roadmap, scope-expansion, or future-design work. Skip for
ordinary implementation and user documentation of available behavior.

## Status boundary

These directions are proposals, not implemented capabilities, approved syntax,
or permission to expand an unrelated task. The current compatibility and maturity
baseline remains in [hydra-mvp-context.md](hydra-mvp-context.md).
Versions are assigned to complete changes according to compatibility, not reserved
in advance as a post-1.0 roadmap of 0.x releases.

## Deferred capabilities

| Direction | Boundary |
|---|---|
| Public editor schema through SchemaStore | No `$schema` annotation until a stable public schema exists |
| Shareable Head Recipes | Shelved while Hydra remains local; safe completion would require shared lifecycle coordination |
| Head setup | Open hypothesis focused on per-Head Docker coexistence; generic bootstrap value is not demonstrated |
| Agent runtime adapters | Separate from the available instructional Hydra skill |
| Runtime processes, ports, local dashboard, visual diff, embedded terminal | Built only on a reliable Head engine |
| Assisted merge/rebase and interactive resolution | Must not be advertised as current conflict automation |
| Docker, service, database, and cache isolation | Not implied by filesystem Head isolation |
| Cloud collaboration | Not a current priority or core dependency |
| Tauri desktop application | Only if real usage justifies it |
| Virtual filesystem | Only if guaranteed CoW on incompatible volumes becomes necessary |
| Immutable-content hard links | Only for content explicitly immutable and protected read-only; not a general materialization backend |
| Lost-locator reconnection and installation relocation | Future explicit ownership-validated recovery, never silent path reinterpretation |
| Additional crates or persistent content cache | Require demonstrated boundaries or reuse needs; no prebuilt architecture |
| Short command aliases | Potential interfaces, not current syntax contracts |

## Locator reconnection and relocation proposal

The original storage model permits a future repair to reconnect a lost locator
using the ownership marker of a directory explicitly identified by the user.
Moving the parent project or the whole Heads directory requires explicit verified
relocation, not silently resolving a new path. Current repair does not implement
these operations; this proposal does not authorize manual metadata edits or relaxing
project and installation identity checks.

## Shareable Head Recipes

Shareable Head Recipes are not planned in the current product direction. Hydra
remains local-first: each installation owns the lifecycle of its physical Heads,
while Git transports commits and refs between collaborators.

A shared recipe would outlive any one local materialization. Hydra could not
safely remove or complete that artifact when one user closes a Head because
other users might still depend on it. Solving that lifecycle requires shared
completion state, concurrency rules, and cross-installation coordination. Those
capabilities are outside Hydra's current local boundary.

No recipe command, schema, artifact, promotion flow, or shared-close behavior is
approved. A future proposal MUST first justify a shared lifecycle model rather
than treating portable Head intent as a standalone local feature.

## Head setup hypothesis

Head setup remains an open product hypothesis. Current field evidence indicates
that overlays already supply the local files and configuration normally needed
after creation. A generic dependency-installation or bootstrap hook therefore
has no demonstrated recurring value and MUST NOT be treated as the default
design.

The demonstrated gap is per-Head Docker coexistence. Multiple Heads can require
distinct Compose project names, host ports, URLs, volumes, cache prefixes,
database names, or session identities before their stacks can run side by side.
A future setup proposal SHOULD start from explicit, reviewable per-Head Docker
configuration rather than arbitrary implicit hooks. It MUST define failure,
retry, cleanup, configuration ownership, secret handling, and interaction with
overlays before approval. Hydra still isolates Git and files only; this proposal
does not imply current container, service, database, port, or cache isolation.

## Product hypothesis and admission

The hypothesis is that complete isolated Git Heads reduce interference between
parallel development activities while preserving review and integration control.
A future session model may add IDE, agent, or runtime context, but the core remains
ordinary directories, independent diffs, isolated content, and Git compatibility.

Before promoting a proposal, obtain an explicit scoped product decision, define
success and refusal behavior, inspect current implementation, and add TDD coverage.
Update canonical contracts, affected routers, user documentation, and the Hydra
skill only as the capability becomes implemented and verifiable. History retains
old examples; do not revive them as supported commands.

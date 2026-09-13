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
| Shareable Head Recipes | Portable intent, not a versioned physical Head |
| Setup command or hooks | Requires explicit lifecycle and failure contracts |
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
| Short command aliases and JSON output | Potential interfaces, not current syntax contracts |

## Locator reconnection and relocation proposal

The original storage model permits a future repair to reconnect a lost locator
using the ownership marker of a directory explicitly identified by the user.
Moving the parent project or the whole Heads directory requires explicit verified
relocation, not silently resolving a new path. Current repair does not implement
these operations; this proposal does not authorize manual metadata edits or relaxing
project and installation identity checks.

## Head Recipe proposal

A physical Head is local and not versioned. A future recipe could be authored
directly or created by promoting a local Head through a dedicated command. It
could transport a reproducible source, target, name, overlay profile, and lifecycle
intent to another device. It MUST NOT transport local absolute paths, backend selection, locks,
operational timestamps, uncommitted content, or overlay secrets.

Promotion of a local Head would require proving that shared content is reachable
through Git. Each recipient would materialize its own worktree, private branch,
path, and storage backend. Git transports commits and refs; the recipe transports
intent. No recipe command or schema is currently available.

A future ephemeral recipe could request deletion after successful close. Because
a versioned recipe is a Git file, that deletion would need an explicit close
transaction: no dirtying another worktree silently, no deletion after failed close,
and no unauthorized implicit commit.

The original design sketch names the intended fields below. This is a proposed
shape, not an accepted schema, supported configuration, or executable workflow:

```json
{
  "version": 1,
  "name": "payment",
  "source": "feature/payment",
  "target": "main",
  "overlayProfile": "default",
  "lifecycle": {"removeRecipeOnClose": true}
}
```

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

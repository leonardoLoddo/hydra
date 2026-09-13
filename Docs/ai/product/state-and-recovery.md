# Local State and Recovery Contract

**Status:** current
**Scope:** ownership identity, persisted intent, locking, interruption, and reconciliation
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read for persisted state, ownership, schema compatibility, locks, repair, or
interruption behavior. Skip for changes without state or recovery implications.

## Inherited defaults

Load Head isolation and recoverability from
[hydra-mvp-context.md](hydra-mvp-context.md#default-head-isolation-and-recoverability).
Recovery MUST preserve Git work and MUST NOT invent missing intent.

## Authority and storage

| Artifact | Role | Versioned in the project |
|---|---|---|
| `.hydra.json` at the canonical parent | Shared project policy | Yes |
| `<git-common-dir>/hydra/project.json` | Local locator: project and installation identities, canonical parent and Heads paths | No |
| `<heads-directory>/.hydra/directory.json` | Ownership marker matching the locator | No |
| `<heads-directory>/.hydra/heads.json` | Physical local Head inventory | No |
| `pending-<name>.json` in the Heads metadata directory | Exact interrupted creation intent | No |
| `recovery-<name>.json` in the Heads metadata directory | Central recovery evidence | No |
| `hydra-head.json` in the private linked-worktree Git directory | Independent copy of exact recovery evidence | No |

`projectId` identifies the shared project across devices and MUST NOT depend only
on its directory name. Collaborators share it through configuration, while each
local initialization has its own locator, physical Heads directory, and
`installationId`. Same-named repositories at different paths are not interchangeable.

A matching project identity with a different installation identity does not
establish ownership. Locator and marker MUST agree before mutation. Local paths
are canonicalized and validated against the owned directory and registered
worktrees. Malformed, symlinked, or unsupported state MUST be preserved as an
error, not silently migrated or replaced.

Inventory records preserve `name` as the map key and `worktreePath`, `headRef`,
`baseRef`, `baseCommit`, `targetRef`, `materializationBackend`, and `createdAt`.
The base commit is exact; the base ref is intent. Git cannot recreate every field.
Hydra MUST NOT store proprietary patches required to reopen a Head, or make
committed work recoverability depend solely on this inventory. PID, ports,
agents, and runtime sessions are outside current persisted-state scope.

Local state currently uses version 1; shared configuration uses version 2.
Compatibility promises apply to distributed contracts, not unreleased
experimental formats. Schema changes require explicit compatibility treatment.

## Mutation and interruption

Cooperating mutations serialize through an OS guard on the stable ownership
marker and a versioned ephemeral `heads.json.lock`. The ownership JSON remains
immutable. Ordinary lifecycle commands MUST NOT steal or delete a pre-existing
lock. Read-only inspection does not acquire a mutation lock.

State publication MUST be atomic. Workflows validate before mutation, preserve
exact ownership of newly created artifacts, and distinguish pre-publication
rollback from post-publication cleanup failures. They MUST NOT remove a committed
Head to compensate for failed lock cleanup or directory synchronization.

After a process interruption, an abandoned current-format marker is distinguished
from an active lock by reacquiring the OS guard, not by guessing from a PID.
Malformed and unsupported lock formats are errors. Active locks are preserved.

Atomic file visibility does not prove identical power-loss durability on every
platform. Initialization interrupted before complete ownership publication and
creation interrupted after worktree registration but before recovery publication
can remain report-only; do not promise automatic recovery for those states.

## Guided repair

`hydra repair` plans read-only from Git, filesystem, and validated metadata.
No repair is applied without explicit confirmation and revalidation. The current
repair classes are:

- remove an abandoned current-version lock after reacquiring its guard;
- reconstruct a missing inventory from a complete set of exact recovery records;
- adopt an omitted registered Head with matching recovery evidence;
- clean safe interrupted pre-worktree creation intent;
- remove stale inventory entries while preserving private branches;
- restore an unambiguously relocated worktree to its original managed path.

Missing-inventory recovery requires at least one valid central or private record
for every expected Hydra-prefixed worktree. If both exist, they MUST match exactly.
Refusal, missing evidence, or inconsistent records MUST NOT publish a partial
inventory. Malformed inventories are never replaced by this recovery action.
Adoption preserves existing entries and requires the complete approved candidate
set to remain unchanged under the mutation lock.

Pending-intent cleanup may delete a private branch only when no associated path
or worktree exists and the branch is absent or still at its recorded base commit.
Deletion uses compare-and-swap. Advanced branches, present paths, registered
worktrees, and ambiguous records remain preserved. A journal left after committed
creation can be removed without changing the registered Head.

Relocation repair moves the worktree back to the managed path; it does not retarget
ownership toward arbitrary paths. Stale inventory removal requires absent path,
absent registered worktree, and an existing private branch that remains preserved.
After lock removal or adoption, rerun repair to plan remaining issues afresh.

Repair does not edit tracked code, relocate an entire installation, reconstruct
lost shared policy, repair ownership identity, remove active locks, or guess missing
Head metadata. Report-only cases require diagnosis, not unsafe manual metadata edits.

## Evidence and verification

Inspect `crates/hydra-core/src/head/state.rs`, `head/persistence.rs`,
`head/recovery.rs`, and `head/repair/` under the same source root.
Run CLI `repair`, `head_create_state_failures`, and `head_remove` tests. Verify
exact restored inventory, preserved refs and bytes after refusal, unchanged active
locks, complete-set cancellation after races, and recoverable partial failures.
Detailed applicability and the Windows read-only lock exception belong to
[../architecture/repair.md](../architecture/repair.md).

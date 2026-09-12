# Hydra Product Contract

**Status:** current
**Scope:** product identity, isolation guarantees, Git compatibility, and core acceptance criteria
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read for product direction, Head semantics, safety guarantees, or scope decisions.
Skip for editorial changes and internal changes that cannot affect these contracts.

## Product boundary

Hydra is a local-first, Git-native workspace manager. A Head is a complete,
independent directory for one development activity. People, IDEs, terminals,
and AI agents use ordinary files and Git without a Hydra-specific integration.

Hydra coordinates isolated workspaces. It is not an IDE, AI agent, Git
replacement, cloud environment, full virtual machine, or runtime process manager.
The core requires no cloud service or daemon. It does not isolate databases,
caches, containers, external services, or installed dependencies automatically.

## Canonical terms

| Term | Meaning |
|---|---|
| Head | One local managed Git worktree with its own private branch and writable files |
| parent project | Canonical project root recorded by the local installation locator |
| private branch | Ref advanced by one Head; several Heads may share the same starting commit |
| baseRef | Original source intent, normalized when symbolic |
| baseCommit | Exact commit resolved at creation; never an automatically advancing snapshot |
| targetRef | Existing local branch selected for explicit integration |
| overlay | Selected local untracked content from the canonical parent project |
| Materializer | Boundary that creates isolated visible files using native CoW or full copy |
| CoW | Filesystem copy-on-write; logically independent files initially sharing physical blocks |

## Default: Head isolation and recoverability

**Applies to:** all Hydra project and Head workflows, storage backends, and adapters.

Hydra MUST preserve separate working trees, indexes, `HEAD`s, private branches,
uncommitted changes, and diffs. Two Heads MUST NOT operate on the same private
ref. Multiple Heads MAY start from one branch or commit and integrate into one
target through explicit actions.

Writing a tracked file or overlay in one Head MUST NOT alter another Head or
the parent project's operational files. Sharing the Git object database is
allowed; mutable hard links are forbidden. Storage efficiency MUST NOT weaken
write isolation. A full-copy fallback MUST be visible rather than presented as CoW.

A Head starts at `baseCommit`. Hydra MUST NOT silently follow an advancing base,
rebase, push, pull, synchronize, or resolve conflicts. Explicit `head close`
integration and protected branch cleanup are governed by
[lifecycle.md](lifecycle.md); ordinary direct Git commands remain available.

Commits created in a Head are immediately available in the shared repository.
Loss of Hydra metadata MUST NOT make committed work dependent on a proprietary
patch format or prevent normal Git recovery. Git owns refs, commits, worktree
registration, and observed changes. Hydra records intent Git cannot reconstruct;
[state-and-recovery.md](state-and-recovery.md) defines the recovery boundary.

Hydra MUST validate paths, ownership, refs, and external state before mutation.
It MUST NOT traverse outside approved roots, silently force deletion, or use
unescaped shell construction. Multi-step mutations MUST retain rollback ownership
or enough exact evidence for reconciliation. Ambiguous data MUST be preserved.

Configured adapters are trusted user programs, not sandboxed code. Hydra owns
argument separation and its protected lifecycle steps; it cannot guarantee or
undo arbitrary external effects. See [lifecycle.md](lifecycle.md).

## Compatibility and maturity

Version `1.0.0` establishes the 1.x compatibility baseline for documented CLI,
configuration, persisted state, and Head lifecycle. Incompatible changes require
a new major release. Public-preview maturity is a separate product decision:
version 1.x does not prove complete field validation on every supported platform.
Release mechanics inherit this baseline through the Development route.

## Core acceptance criteria

Changes MUST preserve these observable outcomes:

- Initialize a real Git repository with safe external Heads placement and live
  `.gitignore` overlay policy.
- Create at least three Heads from one commit with distinct private branches,
  indexes, normal files, and independent diffs; commits remain visible to Git.
- Materialize tracked content and overlays with verified CoW or isolated copy,
  honor rule precedence, and reject forbidden paths and mutable hard links.
- Inspect Head state accurately without implicit repair; report the actual
  storage backend through a real destination-volume probe.
- Open configured tools, integrate explicitly, remove safely, and retain
  unintegrated commits under the documented force and failure boundaries.
- Reconcile only the state supported by exact ownership and recovery evidence.
- Preserve current shell completion, English and Italian user documentation,
  and the installable operational Hydra skill.
- Verify lifecycle success, refusal, interruption, recovery, and isolation on
  disposable repositories; distinguish native-platform evidence from assumptions.

## Evidence and verification

Inspect `crates/hydra-core/src/head.rs`, `crates/hydra-core/src/init.rs`, and
`crates/hydra-cli/tests/` for implemented boundaries. Run the applicable lifecycle
integration tests and check actual Git refs, index, working files, and metadata.
The Architecture route records specific failure tests and implementation gaps.
An acceptance criterion is normative intent, not a claim that every platform or
failure mode has been verified.

## Related knowledge

Use the [product router](ROUTER.md) to select configuration, storage, lifecycle,
state, or CLI contracts independently. Future capabilities are isolated in
[roadmap.md](roadmap.md), which is not current implementation guidance.

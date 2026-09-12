# Project Configuration and Overlays

**Status:** current
**Scope:** portable shared policy and local overlay selection
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read when changing .hydra.json, directory strategies, storage settings, overlay
selection, included rules, or exclusion confirmation. Skip for unrelated lifecycle changes.

## Inherited defaults

Load the Head isolation and recoverability default in
[hydra-mvp-context.md](hydra-mvp-context.md#default-head-isolation-and-recoverability).
The local protections below extend it; none authorize escaping the project or
sharing mutable files.

## Shared configuration

The canonical parent project's `.hydra.json` is versioned shared policy. A
calling Head's absent, stale, or modified copy MUST NOT redefine that policy.
Physical local paths belong in the locator, not versioned configuration.

The supported configuration is strict JSON schema version 2. Unknown fields,
unknown variants, missing required fields, and variant-inappropriate fields are
errors. The unreleased experimental version 1 is rejected without migration.
Comments and `$schema` are not accepted; editor schema publication is proposed.

The initial policy contains:

```json
{
  "version": 2,
  "projectId": "example-project",
  "headsDirectory": {"strategy": "sibling", "suffix": ".heads"},
  "branchPrefix": "hydra/",
  "storage": {"mode": "auto"},
  "overlay": {"copy": ["... .gitignore"]}
}
```

`projectId` is generated once and persisted; the example is not its generation
algorithm. Callers treat the identifier as opaque.

| Directory strategy | Required policy | Resolution |
|---|---|---|
| `sibling` | `suffix` | `<repository-parent>/<repository-name><suffix>` |
| `relative` | `base: "repositoryParent"`, `path` | Portable path below the canonical repository parent |
| `local` | No path field | Absolute path in the non-versioned locator |

The default is the sibling suffix `.heads`. Heads MUST remain outside the parent
working tree and every other worktree. Hydra MUST NOT reuse another project's
owned directory or create nested Heads. This avoids recursive materialization,
accidental tracking, and indexing every Head from the parent project.

A suffix is a non-empty filename fragment. Unicode, spaces, and punctuation are
allowed; control characters, `/`, and `\` are rejected. There is no required
leading separator. Platform filename failures remain explicit operational errors.
A relative path uses `/`, is non-empty and non-absolute, and has no `.` or `..`
components. A `local` strategy requires an explicit local destination; the
current CLI does not provide a general relocation or metadata-editing workflow.
All resolved destinations still require canonical ownership validation.

`storage.mode` accepts `auto` and `copy`. `auto` prefers verified CoW; `copy`
forces full copies of regular tracked and overlay files. Explicit copy mode does
not authorize overlay storage cost: fallback confirmation still applies.

Optional `commands.open` and `commands.close` are governed by
[lifecycle.md](lifecycle.md). Configuration rewrites MUST preserve supported
optional fields and their absence.

## Overlay selection

Tracked files come from `baseCommit`. Overlays always come from the canonical
parent project's local working files, including when creation runs from a Head.
They are local content, not a mechanism to distribute secrets or runtime state.

`overlay.copy` uses Gitignore matching semantics: `*`, `?`, `**`, root-anchoring
slashes, directory-ending slashes, `!` negation, escaping, order, and last matching
rule wins. A positive match selects a copy; a negated match excludes it. Do not
replace this with generic filesystem glob semantics.

`... <relative-file>` expands rules in place. The default `... .gitignore`
keeps `.gitignore` live instead of copying its contents into configuration.
Comments in expanded files follow Gitignore syntax. An absent included file adds
no rules. An existing included file MUST be regular and resolve through safe
relative components. Expansion MUST NOT recurse. The implementation expands
configuration directives once and reads the included file as Gitignore rules;
it does not recursively interpret Hydra include directives from those files.
Unsafe included paths are rejected.

Only existing selected entries are materialized. Planning records deterministic
path order, logical size, regular-file identity, and symlink targets. Hashing
MUST preserve path-to-content association and use bounded batches and workers
rather than one Git process per file. The Materializer verifies destination
content against the plan and fails safely if concurrent source edits change it.

## Mandatory overlay protections

Hydra MUST NOT copy `.git`, the Heads directory, special files such as sockets
or pipes, or overlay content that overwrites a tracked entry. It MUST NOT follow
symlinks outside the source root. Relative overlay symlinks are preserved as
links only when their final targets stay inside both source and resulting Head.
Absolute, broken, escaping, and platform-unsupported links are rejected.

When initial planning finds unsafe overlay symlinks, Hydra MAY offer explicit
persistent exclusion of those exact paths. It MUST collect all such paths in
stable order, present them, and default to refusal. Only affirmative consent
allows appending literal root-anchored negations to `overlay.copy` before any Git
mutation. Other unsafe entries and links that become unsafe after mutation remain
errors, not candidates for this exception.

**Effect:** the exclusion workflow extends the rejection rule with an explicitly
authorized policy correction; it never authorizes copying the rejected links.

Refusal, EOF, or unrecognized input leaves configuration, refs, worktrees, and
inventory unchanged. Accepted exclusions remain visible versioned changes even
if a later full-copy prompt is declined or creation fails. They require review;
Hydra does not create an implicit commit.

Configuration replacement is atomic but is not a portable content compare-and-swap.
Hydra rejects edits visible at its final comparison, but an external editor can
still race the following rename. Users and agents MUST avoid editing configuration
during this confirmation and inspect the resulting diff. The state lock serializes
cooperating Hydra mutations, not arbitrary editors.

Every non-empty overlay plan reports file count and logical bytes. If all actual
source-to-destination CoW probes succeed, creation proceeds without a cost prompt.
If any regular overlay requires full copy, Hydra shows that subset's count and
bytes and requires confirmation before branch, worktree, or inventory creation.
An unexpected later CoW failure requires rollback and confirmation, not silent copy.

## Evidence and verification

Inspect `crates/hydra-core/src/head/state/configuration.rs`,
`crates/hydra-core/src/head/overlay.rs`, and
`crates/hydra-core/src/head/overlay/materialization.rs`.
Run configuration unit tests and CLI `head_create_conflicts`,
`head_create_overlay_failures`, `head_create_state_failures`, and
`head_create_success` integration targets. Verify source isolation, strict schema
rejection, path safety, rule order, accepted exclusions, refused prompts, and
unchanged state on errors. Creation transaction mechanics belong to
[../architecture/head-creation.md](../architecture/head-creation.md).

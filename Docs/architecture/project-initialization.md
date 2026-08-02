# Project Initialization

## Purpose

This document defines how the current implementation realizes:

```text
hydra init [path]
```

It owns the technical workflow, persistence order, failure behavior, and
implementation constraints of project initialization. The intended
user-visible contract remains authoritative in
[`../product/hydra-mvp-context.md`](../product/hydra-mvp-context.md).

When this document records an implementation gap, the product requirement is
not relaxed. The gap must be closed in code and tests or resolved through an
explicit product decision.

---

## CLI Contract

The executable exposes:

```text
Usage: hydra init [PATH]
```

`PATH` defaults to `.`. It may identify the repository root or a path contained
by a Git working tree. The CLI delegates initialization to `hydra-core`, prints
a success message to stdout, and returns a successful exit status only after
the core workflow completes.

Operational errors are written to stderr and produce a non-zero exit status.
The CLI does not perform partial initialization itself.

`hydra --version` reports the executable name and the workspace package
version. After successful initialization, stdout identifies both the repository
and the storage backend verified on the Heads volume:

```text
Initialized Hydra in <repository-root>
Storage backend: copy-on-write
```

The second line is `Storage backend: full copy` when the native clone probe is
not supported and the safe copy fallback succeeds.

---

## Repository Discovery

Initialization resolves repository paths using Git rather than assuming that
the supplied path is the repository root.

The current adapter executes these commands with separately passed arguments:

```text
git -C <path> rev-parse --path-format=absolute --show-toplevel
git -C <repository-root> rev-parse --path-format=absolute --git-common-dir
```

The first result identifies the working-tree root where `.hydra.json` belongs.
The second identifies the shared Git administrative directory where local Hydra
state belongs. This distinction is required for repositories accessed through
Git worktrees.

Hydra rejects the operation before mutation when Git cannot start, Git returns
a failing status, output is empty, or the resolved repository path has no
usable name and parent directory. Git failures retain the operation being
performed, exit status, and stderr so an unsupported Git option is not
misreported as a missing repository.

On Unix, Git path output is converted without requiring UTF-8 and only the
single record terminator emitted by Git is removed. Repository names must still
be valid UTF-8 because they are persisted in JSON and used in the sibling
directory name. Unsupported names are rejected before mutation instead of
being converted lossily. Non-Unix builds require Git path output to be UTF-8.

---

## Derived Paths

For a repository:

```text
<parent>/<repository-name>/
```

the current implementation derives:

| Artifact | Path |
|---|---|
| Shared project configuration | `<repository-root>/.hydra.json` |
| Default Heads directory | `<repository-parent>/<repository-name>.heads/` |
| Local locator directory | `<git-common-dir>/hydra/` |
| Local project locator | `<git-common-dir>/hydra/project.json` |
| Heads metadata directory | `<heads-directory>/.hydra/` |
| Directory ownership marker | `<heads-directory>/.hydra/directory.json` |
| Local Head inventory | `<heads-directory>/.hydra/heads.json` |

The shared configuration stores a portable `sibling` policy. The local locator
stores canonical absolute `projectRoot` and `headsDirectory` paths so every
linked worktree can bootstrap from the same Git common directory without
resolving the policy against its own root.

### Canonical lifecycle context

After initialization, every lifecycle command first discovers the caller's Git
common directory and reads its local locator. Hydra then validates that the
locator's canonical `projectRoot` belongs to that same Git common directory and
uses the resulting parent repository as the command context.

Consequently, invoking Hydra from a managed Head is equivalent to invoking it
from the parent project for configuration loading, Git defaults, overlay
sources, project reporting, and state ownership. The calling Head does not need
to contain `.hydra.json`, and its private branch, working files, or stale copy
of the configuration cannot redefine the project context. Adapters that
explicitly operate on a selected Head still run in that validated Head path.
Re-running `hydra init` from a managed Head resolves the same parent and reports
that parent's configuration as already initialized; it never attempts to
initialize the Head as a separate project.

---

## Initial Persisted Data

The project configuration uses Hydra's project schema version 2:

```json
{
  "version": 2,
  "projectId": "example-5b8d9f430d5543eca3aa967dd484bf41",
  "headsDirectory": {
    "strategy": "sibling",
    "suffix": ".heads"
  },
  "branchPrefix": "hydra/",
  "storage": {
    "mode": "auto"
  },
  "overlay": {
    "copy": [
      "... .gitignore"
    ]
  }
}
```

Hydra does not currently publish an editor schema, and initialization neither
generates nor accepts a `$schema` annotation. Runtime deserialization remains
the authoritative trust-boundary validation and rejects every unknown
top-level field. Publishing the configuration schema through SchemaStore is a
planned follow-up; editor annotations must not return before that stable
public distribution exists.

The Git common directory receives the local locator:

```json
{
  "version": 1,
  "projectId": "example-5b8d9f430d5543eca3aa967dd484bf41",
  "installationId": "local-7e8864fe35434de98d762a73387fbd76",
  "projectRoot": "/work/Example",
  "headsDirectory": "/work/Example.heads"
}
```

The Heads directory receives a matching ownership marker:

```json
{
  "version": 1,
  "projectId": "example-5b8d9f430d5543eca3aa967dd484bf41",
  "installationId": "local-7e8864fe35434de98d762a73387fbd76"
}
```

Its physical inventory is initialized as:

```json
{
  "version": 1,
  "heads": {}
}
```

All four files are pretty-printed UTF-8 JSON terminated by a newline. Version-1
project configurations are intentionally unsupported because the experimental
format was never released.

### Project identifier

The current `projectId` is generated once during initialization and becomes
stable through persistence in `.hydra.json`.

Its implementation format is:

```text
<repository-slug>-<32 random hexadecimal characters>
```

The slug:

- lowercases ASCII alphanumeric characters;
- replaces other characters with `-`;
- collapses repeated `-`;
- removes leading and trailing `-`;
- falls back to `project` when no slug characters remain.

The random suffix comes from a UUID version 4. Callers must treat the complete
identifier as opaque; its formatting is not a user-facing compatibility
guarantee.

`installationId` is generated independently as `local-` followed by the 32
hexadecimal UUID characters. It identifies only this local initialization and
must match between the locator and directory marker.

---

## Validation Before Mutation

Initialization validates all predictable destination conflicts before creating
an artifact.

It refuses to proceed when any filesystem entry, including a dangling symlink,
already occupies:

- `.hydra.json`;
- the default sibling Heads directory;
- `<git-common-dir>/hydra`.

The existing Heads directory is never claimed, emptied, or reused implicitly.
The same rule applies to the local locator directory: initialization creates
it exclusively and never follows or claims a pre-existing directory or symlink.
This protects unrelated data and removes the check-then-use window for that
trust boundary while ownership reconciliation is not yet implemented.

Serialization of all four JSON documents also completes in memory before the
first filesystem mutation.

---

## Persistence Transaction

After discovery, validation, and serialization, the implementation performs:

```text
create Heads directory
        ↓
probe clone and full-copy capability in Heads directory
        ↓
create <heads-directory>/.hydra exclusively
        ↓
create <git-common-dir>/hydra exclusively
        ↓
atomically publish directory.json
        ↓
atomically publish <heads-directory>/.hydra/heads.json
        ↓
atomically publish <git-common-dir>/hydra/project.json
        ↓
atomically publish .hydra.json
        ↓
report success
```

`.hydra.json` is published last. Its presence therefore means that all earlier
steps returned successfully during the same process.

The storage probe creates uniquely named source and target files inside the new
Heads directory. It attempts the platform clone primitive through the
`reflink-copy` safe API and verifies the target bytes. If cloning fails, it
creates the target exclusively, performs a full copy, synchronizes it, and
verifies the bytes. Every probe file is removed before initialization
continues; cleanup failure aborts initialization and is reported.

Each JSON file is published with this sequence:

1. create a uniquely named temporary file in the destination directory using
   exclusive creation;
2. write the complete serialized content;
3. call `sync_all` on the temporary file;
4. create the final path as a hard link to the completed temporary file;
5. remove the temporary link;
6. on Unix, synchronize the parent directory.

The hard-link publication step is an atomic create-if-absent operation: it
cannot replace a destination introduced between validation and publication.
The temporary and final entries refer only to the same completed, immutable
metadata payload and the temporary entry is immediately removed. Mutable Hydra
or application files must never use hard links as an isolation mechanism.

---

## Rollback Ownership

Rollback removes only artifacts created by the current invocation.

| Failure point | Required cleanup |
|---|---|
| Storage probe | Remove probe files and the newly created empty Heads directory |
| Creating either metadata directory | Remove only directories created by the invocation, from child to parent |
| Publishing marker or inventory | Remove previously published owned files and the empty metadata directories |
| Publishing the locator | Remove marker, inventory, and empty metadata directories |
| Publishing project configuration | Remove locator, marker, inventory, and empty metadata directories |
| Temporary-file write or publication | Remove every temporary and final link owned by the failed publication |

Cleanup is deliberately limited to `remove_file` and `remove_dir` on exact
paths. `remove_dir` succeeds only for an empty directory, so rollback cannot
recursively erase unexpected content.

If cleanup fails, the returned error contains both the original operational
error and every exact path that rollback could not remove. A cleanup failure is
therefore diagnosable and never silently hidden.

---

## Error Model

`hydra-core` returns typed initialization errors for:

- Git executable failure;
- Git command failure with operation, status, and stderr;
- empty or unsupported Git path output;
- repository path without a usable parent or name;
- repository name that cannot be persisted losslessly;
- existing project configuration;
- existing Heads directory;
- existing or unsafe local locator directory;
- failed or invalid storage capability probe;
- JSON serialization failure;
- contextual filesystem failures.

Filesystem and serialization errors preserve their original error source. The
CLI renders the error but does not reinterpret it or retry mutations.

---

## Verification Contract

Initialization behavior is protected through CLI integration tests that launch
the compiled `hydra` executable against unique temporary directories and real
temporary Git repositories.

Coverage currently proves:

- the documented `hydra init [PATH]` syntax;
- executable version output and internal Clap command consistency;
- defaulting the optional path to the current directory;
- rejection outside a Git repository without creating `.hydra.json`;
- creation of version-2 configuration, sibling Heads directory, locator,
  ownership marker, and physical inventory;
- real storage probing with visible backend selection and verified full-copy
  fallback;
- schema versions and initial configuration values;
- full UUID entropy in the generated project and installation identifiers;
- correct use of the Git common directory from a linked worktree;
- refusal to reuse a pre-existing default Heads directory;
- refusal to claim or follow a pre-existing local locator directory;
- preservation of dangling configuration symlinks;
- preservation of trailing whitespace in repository paths;
- distinct diagnostics for unrelated Git command failures;
- absence of newly created configuration and local metadata after destination
  conflicts;
- removal of the new Heads directory when local metadata creation fails;
- atomic no-clobber publication;
- explicit diagnostics when rollback cannot remove an owned artifact.

Tests must continue to assert externally observable files, exit status, stdout,
and stderr rather than only internal calls.

---

## Known Implementation Gaps

The following product requirements are not yet implemented by the current
initialization workflow:

1. **Recognition of an owned existing Heads directory.** The current
   implementation safely rejects every pre-existing default directory. It
   cannot yet distinguish a directory owned by the same project from unrelated
   data during `hydra init`; ownership is validated when opening an existing
   installation for Head creation.
2. **Crash reconciliation.** File publication is atomic, but an external
   interruption between transaction steps can leave local metadata files or
   empty directories without `.hydra.json`. Initialization does not yet resume
   or reconcile that state.
3. **Non-Unix directory durability.** Unix parent directories are synchronized
   after metadata publication. The non-Unix implementation currently provides
   atomic visibility but does not claim the same power-loss durability.
4. **Non-UTF-8 repository names.** Unix Git paths are preserved byte-for-byte,
   but a repository name that cannot be represented in JSON is rejected rather
   than encoded into a persistent surrogate.

These gaps are implementation boundaries, not accepted reductions of the MVP
safety or completion criteria.

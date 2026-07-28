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
a failing status, output is empty or not valid UTF-8, or the resolved
repository path has no usable name and parent directory.

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
| Local state directory | `<git-common-dir>/hydra/` |
| Local Head state | `<git-common-dir>/hydra/heads.json` |

The stored `headsDirectory` value is relative to the repository root:

```text
../<repository-name>.heads
```

The resolved Git common directory, not an assumed `.git` subdirectory, must be
used for local state.

---

## Initial Persisted Data

The project configuration is JSON schema version 1:

```json
{
  "version": 1,
  "projectId": "example-1a2b3c4d",
  "headsDirectory": "../Example.heads",
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

The local state is initialized as:

```json
{
  "version": 1,
  "heads": {}
}
```

Both files are pretty-printed UTF-8 JSON terminated by a newline.

### Project identifier

The current `projectId` is generated once during initialization and becomes
stable through persistence in `.hydra.json`.

Its implementation format is:

```text
<repository-slug>-<8 random hexadecimal characters>
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

---

## Validation Before Mutation

Initialization validates all predictable destination conflicts before creating
an artifact.

It refuses to proceed when:

- `.hydra.json` already exists;
- the default sibling Heads directory already exists;
- `<git-common-dir>/hydra/heads.json` already exists.

The existing Heads directory is never claimed, emptied, or reused implicitly.
This protects unrelated data while ownership reconciliation is not yet
implemented.

Serialization of both JSON documents also completes in memory before the first
filesystem mutation.

---

## Persistence Transaction

After discovery, validation, and serialization, the implementation performs:

```text
create Heads directory
        ↓
create/reuse <git-common-dir>/hydra
        ↓
atomically publish heads.json
        ↓
atomically publish .hydra.json
        ↓
report success
```

`.hydra.json` is published last. Its presence therefore means that all earlier
steps returned successfully during the same process.

Each JSON file is published with this sequence:

1. create a uniquely named temporary file in the destination directory using
   exclusive creation;
2. write the complete serialized content;
3. call `sync_all` on the temporary file;
4. rename the temporary file to its final path;
5. remove the temporary file on a reported failure.

Writing the temporary file beside its destination keeps the rename on the same
filesystem and provides atomic replacement semantics for a destination that
was validated as absent.

---

## Rollback Ownership

Rollback removes only artifacts created by the current invocation.

| Failure point | Required cleanup |
|---|---|
| Creating local state directory | Remove the newly created empty Heads directory |
| Publishing local state | Remove the empty Heads directory; remove the state directory only if this invocation created it |
| Publishing project configuration | Remove the new local state file and empty Heads directory; remove the state directory only if this invocation created it |
| Temporary-file write or rename | Remove the operation's temporary file |

Cleanup is deliberately limited to `remove_file` and `remove_dir` on exact
paths. `remove_dir` succeeds only for an empty directory, so rollback cannot
recursively erase unexpected content.

Rollback is currently best effort: the original operational error remains the
reported error if cleanup itself fails. Future work that introduces
interruption recovery must make leftover artifacts explicitly diagnosable and
reconcilable.

---

## Error Model

`hydra-core` returns typed initialization errors for:

- Git executable failure;
- path not belonging to a Git repository;
- empty or invalid Git path output;
- repository path without a usable parent or name;
- existing project configuration;
- existing Heads directory;
- existing local state;
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
- rejection outside a Git repository without creating `.hydra.json`;
- creation of configuration, sibling Heads directory, and local state;
- schema version and initial configuration values;
- refusal to reuse a pre-existing default Heads directory;
- absence of configuration and local state after that validation failure;
- removal of the new Heads directory when local state creation fails.

Tests must continue to assert externally observable files, exit status, stdout,
and stderr rather than only internal calls.

---

## Known Implementation Gaps

The following product requirements are not yet implemented by the current
initialization workflow:

1. **Storage capability verification.** The product specification requires
   `hydra init` to probe the actual destination volume. The current
   implementation writes `storage.mode: "auto"` but does not execute a CoW
   capability probe. It must not be interpreted as evidence that CoW is
   available.
2. **Recognition of an owned existing Heads directory.** The current
   implementation safely rejects every pre-existing default directory. It
   cannot yet distinguish a directory owned by the same project from unrelated
   data.
3. **Crash reconciliation.** File publication is atomic, but an external
   interruption between transaction steps can leave a local state file or empty
   directory without `.hydra.json`. Initialization does not yet resume or
   reconcile that state.
4. **Rollback diagnostics.** Cleanup is best effort and a cleanup failure is not
   currently added to the returned error.
5. **Directory durability.** Files are synchronized before rename, but parent
   directories are not synchronized after rename. Atomic visibility is
   provided; full durability across sudden power loss is not yet established.
6. **Non-UTF-8 Git paths.** Git path output must currently be valid UTF-8, and
   repository names use a lossy conversion when deriving display-oriented
   names.

These gaps are implementation boundaries, not accepted reductions of the MVP
safety or completion criteria.

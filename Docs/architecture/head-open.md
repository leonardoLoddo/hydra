# Head Open

## Purpose

This document defines the implemented configured adapter workflow:

```text
hydra head open <name>
```

The command validates a recorded Head and starts a user-configured program for
its worktree. Product intent remains authoritative in
[`../product/hydra-mvp-context.md`](../product/hydra-mvp-context.md).

---

## Configuration Contract

The optional schema-v2 project configuration is:

```json
{
  "commands": {
    "open": {
      "program": "code",
      "args": ["{path}"]
    }
  }
}
```

Hydra has no implicit editor or platform-specific default. If `commands.open`
is absent, the command fails without launching a process.

`program` and every element of `args` remain separate process arguments.
Hydra never concatenates them into a shell command. The supported placeholders
are:

- `{name}`;
- `{path}`;
- `{headRef}`;
- `{baseRef}`;
- `{targetRef}`.

Placeholders may occur inside a program or argument template. Braces in
expanded values remain literal and are never interpreted a second time.
Unsupported, unmatched, or literal braces in a template are rejected before
the process starts. Program and argument values containing NUL are also
rejected.

The optional `commands` field is preserved by configuration rewrites, including
guided overlay exclusions. An absent field remains absent.

---

## Validation Boundary

Before starting the configured program, Hydra:

1. validates the logical Head name;
2. validates installation ownership and directory policy;
3. requires the recorded path to equal
   `<owned-heads-directory>/<name>`;
4. requires that path to be a real directory rather than a symlink;
5. requires Git to register that exact path as a worktree;
6. requires the registered symbolic branch to equal `headRef`.

These checks prevent an unsafe inventory edit, missing directory, detached
worktree, or direct Git branch switch from redirecting the configured process.
Opening does not require a clean worktree and does not mutate Git, inventory,
configuration, or Hydra locks.

---

## Process Boundary

The adapter starts with the validated Head path as its current directory.
Standard input, output, and error are inherited, so interactive editor adapters
and their diagnostics behave normally.

Hydra waits for the adapter process. A successful exit prints:

```text
Opened Head payment at /projects/shop.heads/payment
```

Failure to start the program and a non-zero or signal-based exit are command
errors. Hydra preserves the Head and local state in every process outcome.

The configured program is trusted project configuration: it may perform any
action available to the invoking user. Hydra's guarantees cover argument
separation, path validation, and its own lack of hidden lifecycle mutation;
they do not sandbox the external program.

---

## Verification Contract

Disposable-repository CLI tests prove:

- missing `commands.open` fails without state mutation;
- all five placeholders reach the adapter as separate arguments;
- a missing worktree is rejected before the adapter starts;
- a non-zero adapter exit becomes a failed Hydra command without state
  mutation;
- help exposes the nested syntax and configured-command requirement.

Core tests prove that placeholder values may contain literal braces while
unsupported template placeholders are rejected.

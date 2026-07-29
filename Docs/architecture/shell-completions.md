# Shell Completions

## Purpose

This document defines the implemented completion boundary behind:

```text
hydra completions <shell>
```

The user-visible scope remains authoritative in
[`../product/hydra-mvp-context.md`](../product/hydra-mvp-context.md). Validated
inventory loading remains owned by [`head-inspection.md`](head-inspection.md).

---

## Public Contract

Hydra supports dynamic completion registration for Bash, Zsh, and Fish:

```bash
source <(hydra completions bash)
source <(hydra completions zsh)
hydra completions fish | source
```

The generated registration completes the command hierarchy and options. It
also proposes existing Head names for `head status`, `head path`, `head open`,
`head close`, and `head remove`. `head create` deliberately has no Head-name
completer because its positional argument creates a new entity.

An unsupported shell is rejected by the CLI parser with the supported values.
The registration calls the installed `hydra` binary again at completion time,
so users should source it during shell startup and regenerate it after a Hydra
upgrade.

---

## Dynamic Completion Boundary

`hydra-cli` uses `clap_complete`'s environment-activated completion engine
before normal CLI parsing and before any stdout output. Only Bash, Zsh, and
Fish adapters are enabled.

Every existing-Head argument uses the same candidate function. That function:

1. asks `hydra-core::list_heads` for the current directory;
2. treats every discovery, ownership, configuration, or inventory error as an
   empty candidate set;
3. sorts and deduplicates names;
4. filters by the prefix supplied by the completion engine;
5. returns shell-agnostic candidates and leaves escaping to the shell adapter.

The hidden diagnostic contract `hydra __complete heads` prints the same full
candidate set, one name per line. It is not part of public help and exists to
keep the reusable candidate behavior directly testable.

---

## Safety and Performance

Candidate discovery is read-only. It reuses the inventory snapshot loader and
does not acquire the mutation lock, prompt, repair state, start a process, or
invoke a lifecycle mutation. Invalid or unreadable local state therefore
produces successful empty output during interactive completion rather than a
visible error.

The implementation performs one inventory load per candidate request. It does
not inspect each Head's worktree or invoke per-Head Git commands.

---

## Verification Contract

CLI integration tests prove:

- registration output for Bash, Zsh, and Fish;
- parser rejection for unsupported shells;
- real dynamic Bash completion of an existing Head prefix;
- stable sorted and unique candidate output;
- byte-for-byte inventory preservation and absence of the mutation lock;
- successful empty output outside a Hydra project;
- exclusion of the internal candidate command from public help.

The Head-inspection integration tests remain representative coverage for the
shared validated inventory loader.

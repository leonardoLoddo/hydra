# Head Workflows

This page covers the complete supported Head lifecycle. Read
[Core concepts](concepts.md) first if base, target, private branch, or overlay
are unfamiliar.

## Prepare a repository

Hydra operates inside an existing Git working tree. Start with at least one
commit and protect important work:

```bash
cd /path/to/project
git status
git log -1 --oneline
```

Initialization accepts the repository root or a path inside its working tree.

## Initialize Hydra

From the project:

```bash
hydra init
```

Or pass a path explicitly:

```bash
hydra init /path/to/project
```

With the default policy, a project `/workspace/Shop` receives:

```text
/workspace/Shop/.hydra.json
/workspace/Shop.heads/
```

Successful output identifies the canonical repository and the backend verified
on the Heads volume:

```text
Initialized Hydra in /workspace/Shop
Storage backend: copy-on-write
```

The backend may instead be `full copy`. Both provide isolated files. Native
Windows and WSL full-copy results also link to their respective
[Windows](windows-copy-on-write.md) and [WSL 2](wsl-copy-on-write.md) setup
guides.

Review and version `.hydra.json`:

```bash
git diff -- .hydra.json
git add .hydra.json
git commit -m "chore: configure Hydra"
```

Do not add the local locator, ownership marker, inventory, or recovery records
to Git.

If the shared configuration was lost but Hydra finds an exact, empty, locally
owned installation, `init` can recreate only the default `.hydra.json` while
preserving its local identity. It refuses this recovery when Heads, pending
records, recovery records, extra content, or inconsistent ownership exist.
Recover an authoritative configuration from version control instead of asking
Hydra to guess policy for existing Heads.

## Create a Head

Create from the canonical parent project's current `HEAD`:

```bash
hydra head create payment
```

Choose a source and target explicitly when possible:

```bash
hydra head create payment --from beta --target main
```

If `--from` resolves a local branch and `--target` is absent, that local branch
becomes the target. A detached commit or other non-local source requires a
target:

```bash
hydra head create experiment \
  --from 0123456789abcdef \
  --target main
```

Hydra validates names, refs, destination, ownership, configuration, overlays,
and storage cost before creating Git state. A successful result looks like:

```text
New Head successfully created at /workspace/Shop.heads/payment
Storage backend: copy-on-write
```

For non-empty overlays it also prints the logical file count and size. In an
interactive terminal, long operations show phase-level progress on `stderr`;
captured or redirected output does not receive those progress lines.

Creation can ask two default-negative questions before Git mutation:

- whether to exclude unsafe overlay symlinks and update `.hydra.json`;
- whether to continue when some overlay files require a full copy.

On native Windows or WSL, the full-copy prompt prints the applicable setup
guide before `Continue?`. If tracked-file materialization discovers the
fallback without an overlay prompt, successful output prints the same guide
after the effective backend. A single creation prints the guide only once.

Read [Storage and overlays](storage-and-overlays.md) before approving either
prompt. A negative answer or end-of-file safely cancels creation. An approved
symlink exclusion is a durable configuration change even if a later full-copy
prompt is declined, so review and commit it separately.

### Valid Head names

A Head name:

- starts with an ASCII letter or digit;
- continues with ASCII letters, digits, `.`, `-`, or `_`;
- does not contain `..`;
- does not end in `.lock`, case-insensitively.

Valid examples:

```text
payment
auth-v2
issue_123
release.1
```

Invalid examples:

```text
../payment
-payment
auth/refresh
payment.lock
```

The configured `branchPrefix`, `hydra/` by default, is prepended to form the
private branch. The complete branch name must also be valid and unused in Git.

## Enter and work in the Head

Print only the validated absolute path:

```bash
hydra head path payment
```

Use it safely in command substitution:

```bash
cd "$(hydra head path payment)"
git status
git branch --show-current
```

The branch should match the private ref reported by:

```bash
hydra head status payment
```

Edit, build, test, and commit normally inside the Head. Keep task-specific
generated files and uncommitted changes inside this directory.

## Run Hydra from an existing Head

Lifecycle commands work from the parent project or any managed Head. Hydra
uses the shared Git-common locator to load configuration, defaults, overlays,
and inventory from the canonical parent.

For example, from `payment`:

```bash
hydra head create auth
```

creates `/workspace/Shop.heads/auth`, not a directory under `payment`. Without
explicit source or target options, it uses the parent project's current
`HEAD` and local branch. It does not inherit uncommitted files or the private
branch of `payment`.

Running `hydra init` from a managed Head reports the parent as already
initialized; it does not create a second Hydra project.

## List and inspect Heads

List names in stable lexicographic order:

```bash
hydra head list
```

Summarize the project:

```bash
hydra status
```

Each Head is classified as:

- `clean` — no staged, unstaged, deleted, or untracked files;
- `modified` — Git reports working-tree or index changes;
- `inconsistent` — Git, filesystem, or Hydra metadata disagree.

Inspect one Head in detail:

```bash
hydra head status payment
```

The report includes the recorded path, observed branch and commit, exact
creation base, target, change counts, ahead/behind values, worktree presence,
and consistency issues.

Ahead/behind compares the observed worktree commit with the current symbolic
base ref. For a non-symbolic source such as an abbreviated SHA, Hydra always
uses the exact recorded base commit. If a symbolic base disappeared, Hydra
reports the inconsistency and falls back to that exact creation commit for the
comparison.

`hydra status`, `head list`, `head status`, and `head path` are read-only. They
do not take the mutation lock, rewrite metadata, or perform implicit repair.

## Open a Head with a configured program

Hydra has no implicit editor. Add an explicit shared adapter to
`.hydra.json`:

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

Then run:

```bash
hydra head open payment
```

Hydra validates the path, Git worktree registration, and private branch before
starting the process in the Head directory. It waits for the process. A launch
failure or non-zero exit makes the Hydra command fail without modifying the
Head.

The configured program is trusted project code and is not sandboxed. See
[Configuration](configuration.md) for placeholders and argument rules.

## Close and integrate a Head

Native close requires a clean, consistent Head:

```bash
hydra head close payment
```

By default Hydra integrates the private branch into the recorded local target:

- if the target is checked out in a clean registered worktree, Hydra advances
  that worktree while keeping its ref, index, and files synchronized;
- if the target is not checked out, Hydra integrates without checkout;
- it reports `already integrated`, `fast-forward`, or `merge commit`;
- after successful integration, it performs ordinary protected removal.

A checked-out target with staged, modified, deleted, or untracked files, or an
active merge, rebase, or other Git operation, blocks close before mutation.
A merge conflict preserves the target, private branch, Head worktree, and
inventory. Hydra does not resolve conflicts automatically.

Example success:

```text
Closed Head payment into refs/heads/main at <commit>
Integration strategy: target worktree /workspace/Shop
Integration result: fast-forward
```

When no worktree has the target checked out, the strategy is
`checkout-free`.

If integration succeeds but protected removal fails, Hydra reports the target
ref and published commit separately and preserves the remaining Head state.
Do not try to undo the target manually; inspect the Head and run the documented
recovery workflow.

You can run close from the Head being closed. On success, its directory is
removed, so move the shell to the parent project or another surviving Head
before running more commands.

### Configured close command

A project can replace native integration with a trusted command adapter. Hydra
passes separate arguments, waits for the result, and optionally attempts
protected removal. It cannot roll back arbitrary effects such as pushes,
pull-request creation, or external service changes.

See [Configuration](configuration.md) for complete examples and safety rules.

## Remove a Head safely

Ordinary removal requires a clean worktree and a private branch fully
reachable from the recorded target:

```bash
hydra head remove payment
```

On success, Hydra removes the registered worktree, inventory entry, recovery
record, and integrated private branch.

Tracked, staged, untracked, or unintegrated work blocks ordinary removal.

To explicitly discard uncommitted worktree changes:

```bash
hydra head remove payment --force
```

`--force` can permanently delete those uncommitted files. It does not bypass
unsafe paths, ownership mismatches, missing or unregistered worktrees, branch
mismatches, detached state, or a missing target.

If committed work is not integrated, forced removal preserves the private
branch:

```text
Removed Head payment
Preserved branch refs/heads/hydra/payment with unintegrated commits
```

Use normal Git commands to inspect that preserved branch. Do not delete it
until you have intentionally recovered or integrated the commits.

## Diagnose inconsistent lifecycle state

Do not delete directories, branches, inventory entries, or locks manually.
Start with:

```bash
hydra status
hydra head status <name>
hydra repair
```

`repair` first plans from Git and local evidence, then asks before each
deterministic mutation. Many ambiguous conditions are intentionally
report-only. Continue with
[Recovery and troubleshooting](recovery-and-troubleshooting.md).

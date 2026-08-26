# Storage and Overlays

Hydra separates logical isolation from physical storage efficiency. Every Head
must be safe to edit independently; copy-on-write is an optimization negotiated
on the real destination volume.

## Storage backends

With `storage.mode: "auto"`, Hydra attempts:

- APFS clone on compatible macOS volumes;
- `FICLONE` reflink on compatible Linux volumes, including compatible volumes
  mounted in WSL 2;
- isolated full copy when native cloning is unavailable.

Hydra verifies output bytes and never uses mutable hard links as a fallback.
A native primitive can be supported on one volume and unavailable on another,
so operating-system detection alone is insufficient.

The effective Head backend is:

- `copy-on-write` when all regular files were cloned;
- `full copy` when any regular file required the fallback or configuration
  selected `storage.mode: "copy"`.

The backend is recorded per Head in local metadata. Files remain ordinary,
writable paths with either result.

## Diagnose the actual Heads volume

After initialization, run:

```bash
hydra doctor storage
```

Hydra creates a unique temporary directory inside the managed Heads directory,
tests the native primitive, verifies the isolated full-copy fallback, and
cleans up the exact probe files. A successful APFS result looks like:

```text
Storage backend: copy-on-write
Native primitive: APFS clone
Environment: native
Filesystem: unknown
Fallback: full copy (verified)
Mutable hard links: disabled
Isolation: supported
```

When native cloning is unavailable, the report uses `Storage backend: full
copy` and `Native primitive: unavailable`. Linux also reports the filesystem
that owns the Heads directory. A native Windows storage adapter reports
`Windows block clone` only after a real compatible ReFS or Dev Drive probe,
although native Windows is not yet a supported Hydra distribution target.

The command is read-only with respect to Hydra inventory, refs, and worktrees;
it does not take the Head mutation lock. A probe or cleanup failure is still a
failed command and reports any remaining exact path. Do not recursively delete
an unfamiliar leftover path without inspecting it.

## WSL 2 copy-on-write

Hydra installed through Linux Homebrew on WSL 2 uses the Linux `FICLONE`
adapter. WSL itself does not emulate reflinks for every filesystem:

- the distribution's default ext4 root can return `Operation not supported`;
- Windows drives exposed below `/mnt/<drive>` use an interop filesystem such
  as DrvFs/9p and are not a Linux reflink volume;
- an XFS or other reflink-capable Linux filesystem attached to WSL can provide
  copy-on-write when the real Hydra probe succeeds.

Check which filesystems the running WSL kernel can mount:

```bash
grep -w xfs /proc/filesystems
```

Microsoft documents attaching physical disks and VHD files in
[Mount a Linux disk in WSL 2](https://learn.microsoft.com/windows/wsl/wsl2-mount-disk).
Creating, formatting, mounting, and backing up that volume remain explicit
administrator operations outside Hydra.

For the default sibling policy, place both a fresh project clone and its future
Heads directory on the mounted volume before initialization:

```text
/mnt/wsl/hydra-data/Shop
/mnt/wsl/hydra-data/Shop.heads
```

Then initialize and verify the exact destination:

```bash
cd /mnt/wsl/hydra-data/Shop
hydra init
hydra doctor storage
```

Proceed as copy-on-write only when the report says `Storage backend:
copy-on-write`, `Native primitive: Linux reflink`, and identifies the expected
filesystem. Do not edit Hydra's locator to move an existing installation. If a
project was already initialized on ext4 or a Windows mount, make a reviewed
fresh clone on the reflink-capable volume and initialize that clone instead;
assisted relocation is not currently available.

## Force full-copy mode

For deterministic tests or automation:

```json
{
  "storage": {
    "mode": "copy"
  }
}
```

Hydra then skips native clone attempts for regular tracked and overlay files.
Overlay content still requires the normal explicit storage-cost confirmation.

## What an overlay is

Tracked files always come from the selected base commit. An overlay adds
ignored or untracked content from the canonical parent project to the new
Head.

The generated configuration uses:

```json
{
  "overlay": {
    "copy": [
      "... .gitignore"
    ]
  }
}
```

`... .gitignore` expands the current parent project's `.gitignore` rules at
that exact point in the list. An absent expanded file contributes no rules; an
existing one must be a safe regular file.

Hydra uses Gitignore matching semantics:

- `*`, `?`, and `**` wildcards;
- leading `/` to anchor at the repository root;
- trailing `/` for a directory pattern;
- `!` negation;
- comments and escaping from expanded rules files;
- ordered evaluation where the last matching rule wins.

In an overlay selection, a positive ignore-style rule means **copy this
path**, while a negative rule means **do not copy it**.

Example source `.gitignore`:

```gitignore
.env
cache/
!cache/logs/
```

Hydra selects `.env` and ignored content below `cache/`, except the negated
`cache/logs/` paths.

Add explicit rules after the expansion when needed:

```json
{
  "overlay": {
    "copy": [
      "... .gitignore",
      ".tool-cache/",
      "!.tool-cache/private/"
    ]
  }
}
```

## Overlay source and timing

The canonical parent project is the overlay source even when `head create` is
invoked from another managed Head. A sibling Head's local files do not leak
into the new Head.

Hydra plans existing entries, computes content identities, materializes them,
and verifies the destination against the plan. A source change that affects
the observed payload causes creation to fail and roll back instead of
publishing an undeclared partial Head. After a verified copy-on-write clone,
later source changes do not affect the isolated destination.

Overlays are copied only at Head creation. They are not synchronized afterward.

## Protection rules

Hydra rejects:

- `.git` and everything below it;
- absolute paths and path traversal;
- a source that resolves outside the canonical parent repository;
- special files such as sockets and pipes;
- an overlay that would overwrite a tracked entry;
- unsafe expanded-rules paths;
- materialized content that no longer matches the planned identity.

The Heads directory must remain outside the parent repository and every other
registered worktree.

An overlay can contain secrets or machine-specific settings. Hydra does not
make those files safe to commit. Keep secrets ignored and check `git status`
inside every Head.

## Relative symlinks

On macOS and Linux, Hydra preserves a selected relative symlink only when:

- its stored target is relative;
- it resolves inside the canonical source project;
- the recreated link resolves inside the new Head.

Hydra recreates the link rather than dereferencing it. This supports ignored
dependency trees containing launchers such as `node_modules/.bin` or
`vendor/bin` without linking the Head back to the source workspace.

Tracked and overlay symlinks are currently unsupported on non-Unix platforms.

## Unsafe symlink prompt

An absolute, broken, escaping, or platform-unsupported overlay symlink cannot
be safely recreated. Before creating a branch or worktree, Hydra collects all
such paths and may ask:

```text
Unsafe overlay symlinks:
  links/escape
  public/storage
Exclude them and update .hydra.json? [y/N]
```

Only `y` or `yes`, case-insensitively, approves the change. Hydra appends
literal, root-anchored negative rules:

```json
{
  "overlay": {
    "copy": [
      "... .gitignore",
      "!/links/escape",
      "!/public/storage"
    ]
  }
}
```

It atomically publishes the complete configuration and replans. Do not edit
`.hydra.json` while this prompt is open: Hydra detects a change visible at its
final comparison, but portable filesystems cannot provide a content
compare-and-swap against an external save in the final pre-rename window.

After approval, review and commit the shared policy:

```bash
git diff -- .hydra.json
git add .hydra.json
git commit -m "chore: exclude unsafe Hydra overlays"
```

A negative answer, empty input, or end-of-file cancels creation without
changing configuration or Git state. The guided update applies only to unsafe
symlinks found during initial planning; special files, collisions, and paths
that become unsafe later remain errors.

An approved exclusion remains even if you later decline a separate full-copy
prompt, because the policy change was already explicitly authorized.

## Full-copy confirmation

Hydra probes every regular overlay file independently in `auto` mode. If only
some require byte duplication, it reports only that subset:

```text
Full copy required: 2 file(s), 1048576 byte(s)
Continue? [y/N]
```

Only `y` or `yes` approves. Empty input, end-of-file, or any other answer
cancels creation before branch, worktree, or inventory mutation.

The always-printed overlay summary is informational. A prompt appears only
when regular overlay files actually require full-copy fallback or when
`storage.mode: "copy"` selected it intentionally.

## Tracked files and submodules

When the canonical parent working tree's tracked state exactly matches the
base commit, Hydra may use those regular files as copy-on-write sources.
A tracked modification disables that fast path for the complete pass; Hydra
then reads committed blob contents, so local tracked changes never become the
new Head's starting state.

Git submodule entries currently create their directory but do not initialize
or fetch submodule contents. Initialize submodules explicitly inside the Head
when your project requires them. Network access is never implicit.


# WSL 2 Copy-on-Write Setup

Hydra runs as a Linux executable inside WSL 2 and uses the Linux `FICLONE`
reflink primitive. Copy-on-write therefore depends on the Linux filesystem that
actually contains the managed Heads directory; Windows ReFS block cloning is
not used through WSL.

Use this guide when `hydra init`, `hydra doctor storage`, or `hydra head create`
reports:

```text
Storage backend: full copy
Copy-on-write guidance: https://github.com/leonardoLoddo/hydra/blob/main/Docs/user/wsl-copy-on-write.md
```

The full-copy fallback remains isolated and safe. Hydra verifies copied bytes
and never substitutes mutable hard links. This guide is only for reducing
initial storage and I/O costs.

## Why the default location may not provide CoW

- The default ext4 root filesystem in a WSL distribution commonly rejects
  `FICLONE`.
- Windows drives exposed below `/mnt/<drive>` use an interop filesystem such as
  DrvFs or 9p rather than a Linux reflink-capable volume.
- WSL 1 is not supported.

Hydra does not infer capability from the path or filesystem name. Its real
probe on the Heads destination is authoritative.

## Requirements

- a WSL 2 distribution;
- Git and Linux Homebrew with the Hydra Formula installed inside that
  distribution;
- a Linux filesystem with reflink support available to the running WSL kernel,
  such as XFS formatted with reflink support;
- the project and its sibling Heads directory on that same mounted volume.

From PowerShell, confirm the distribution version:

```powershell
wsl -l -v
```

Inside WSL, check whether the running kernel exposes XFS support:

```bash
grep -w xfs /proc/filesystems
```

An empty result means that the example XFS route is not available in that
kernel. It does not authorize installing a kernel module or formatting another
filesystem automatically.

## Prepare a compatible Linux volume safely

Microsoft documents attaching physical disks and virtual hard disks in
[Mount a Linux disk in WSL 2](https://learn.microsoft.com/windows/wsl/wsl2-mount-disk).
Follow the current Microsoft procedure for the exact Windows and WSL versions
in use.

Creating a VHD, partitioning, formatting, or mounting storage can destroy data
when the wrong target is selected. Resolve the exact disk or VHD, preserve
required backups, and obtain administrator or workplace authorization before
performing those operations. Hydra does not create, format, mount, resize, or
back up the volume.

Mount the prepared reflink-capable filesystem at a stable Linux path, for
example:

```text
/mnt/wsl/hydra-data
```

Verify the filesystem that owns that path:

```bash
findmnt -T /mnt/wsl/hydra-data
```

Do not assume that a path below `/mnt/wsl` is suitable merely because of its
name. Confirm the actual filesystem and then rely on Hydra's probe.

## Place the project and Heads together

Create a fresh clone on the mounted Linux volume so the default sibling layout
keeps both locations together:

```text
/mnt/wsl/hydra-data/Shop
/mnt/wsl/hydra-data/Shop.heads
```

Do not move an initialized Hydra installation by editing its locator or local
metadata. If the existing project is on the distribution root or `/mnt/c`,
make a reviewed fresh clone on the new volume and initialize that clone.

## Initialize and verify

Inside WSL:

```bash
cd /mnt/wsl/hydra-data/Shop
hydra init
hydra doctor storage
```

Proceed as copy-on-write only when the real probe reports:

```text
Storage backend: copy-on-write
Native primitive: Linux reflink
Environment: Windows Subsystem for Linux
Filesystem: xfs
Fallback: full copy (verified)
Mutable hard links: disabled
Isolation: supported
```

The filesystem line may name another compatible Linux filesystem. The required
evidence is a successful `Linux reflink` probe on the managed Heads volume.

## If Hydra still reports full copy

1. Run `findmnt -T` against both the project and Heads paths and confirm they
   belong to the intended mount.
2. Keep the project and sibling Heads directory on the same reflink-capable
   volume, especially when overlays are selected.
3. Review `.hydra.json`; `storage.mode: "copy"` deliberately disables native
   cloning.
4. Avoid `/mnt/c` and other Windows interop mounts for this Linux reflink
   workflow.
5. Rerun `hydra doctor storage` and accept CoW only after it reports `Linux
   reflink`.

Do not edit Hydra's locator, ownership marker, inventory, or recovery files to
change the destination. A failed real probe means Hydra will continue using
verified full copies at that location.

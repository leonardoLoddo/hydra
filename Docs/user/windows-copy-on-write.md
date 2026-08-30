# Windows Copy-on-Write Setup

Hydra can use native copy-on-write on Windows when the managed Heads directory
is on a compatible ReFS volume, including a compatible Windows Dev Drive. NTFS
and other unsupported volumes remain safely isolated through full copies.

Use this guide when `hydra init`, `hydra doctor storage`, or `hydra head create`
reports:

```text
Storage backend: full copy
Native primitive: unavailable
Copy-on-write guidance: https://github.com/leonardoLoddo/hydra/blob/main/Docs/user/windows-copy-on-write.md
```

The fallback is safe: Hydra verifies the copied bytes and never substitutes
mutable hard links. This guide is only for reducing initial storage and I/O
costs; it is not required for working-tree isolation.

## Requirements

- native Windows 11 x86-64;
- Git for Windows, with Hydra operated from Git Bash;
- a compatible ReFS volume, preferably a Dev Drive;
- administrator access to create the volume;
- at least 50 GB of free space for a Dev Drive.

Microsoft documents the current requirements and creation choices in
[Set up a Dev Drive on Windows 11](https://learn.microsoft.com/windows/dev-drive/).
An existing volume cannot be converted into a Dev Drive: the designation is
applied when a new volume is formatted.

## Create a Dev Drive safely

Open Windows **Settings**, then select **System > Storage > Advanced storage
settings > Disks & volumes > Create dev drive**.

Windows offers a new virtual hard disk, space released by resizing an existing
volume, or already unallocated disk space. A dynamically expanding VHDX is the
least invasive option when you do not want to repartition a physical disk.
Creating, formatting, resizing, or replacing a volume can affect user data:
review the selected destination, keep required backups, and follow workplace
storage policy before approving the operation.

Dev Drive uses ReFS. ReFS block cloning shares physical regions until a write
allocates an independent region, preserving file isolation. See Microsoft's
[Block cloning on ReFS](https://learn.microsoft.com/windows-server/storage/refs/block-cloning).

## Place the project and Heads together

For the default sibling layout, place a fresh clone and its future Heads
directory on the same Dev Drive:

```text
D:\Projects\Shop
D:\Projects\Shop.heads
```

Keeping both locations on the same ReFS volume allows tracked files and
selected overlay files to use block cloning. A source on another volume can
force individual files to use the safe full-copy fallback, causing the Head's
effective backend to be reported as `full copy`.

Do not move an initialized Hydra installation by editing its locator or local
metadata. When the existing project is on NTFS, make a reviewed fresh clone on
the Dev Drive and initialize that clone instead.

## Initialize and verify

From Git Bash:

```bash
cd /d/Projects/Shop
hydra init
hydra doctor storage
```

Proceed as copy-on-write only when the real probe reports:

```text
Storage backend: copy-on-write
Native primitive: Windows ReFS block clone
Fallback: full copy (verified)
Mutable hard links: disabled
Isolation: supported
```

The drive label, Windows edition, or Dev Drive designation alone is not proof.
Hydra tests the volume that actually contains the Heads directory.

## If Hydra still reports full copy

1. Confirm that the Heads directory is on the intended ReFS volume.
2. Confirm that the project and sibling Heads directory are on the same volume,
   especially when the project uses overlay files.
3. Review `.hydra.json`; `storage.mode: "copy"` deliberately disables native
   cloning.
4. Rerun `hydra doctor storage` from the initialized project.

Do not edit Hydra's locator, ownership marker, inventory, or recovery files to
change the destination. A failed block-clone probe is authoritative for that
location; Hydra continues with verified full copies unless the versioned
configuration requires explicit copy mode.

## Current Windows limitations

- Tracked and selected overlay symlinks are not supported on native Windows.
- PowerShell and Command Prompt can help create the Dev Drive, but the supported
  Hydra workflow uses Git Bash.
- Hydra does not create, resize, format, mount, or mark a Dev Drive as trusted.
- Organization security policy and Microsoft Defender settings remain under
  the user or administrator's control.

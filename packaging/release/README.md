# Hydra release archive

Hydra is a Git-native workspace manager for isolated development Heads.
This archive contains a prebuilt `hydra` executable (`hydra.exe` on Windows)
for the target named in the archive filename; Rust is not required to run it.
Hydra 1.x is the current SemVer compatibility line for the documented core;
distribution remains a public preview while field validation continues.

## Contents

- `hydra` or `hydra.exe`: the command-line executable;
- `completions/hydra.bash` on Windows: the ready-generated Git Bash
  registration;
- `hydra-art.txt`: terminal artwork shown by the Homebrew Formula;
- `skills/hydra/`: the canonical portable Hydra Agent Skill;
- `LICENSE`, `LICENSE-MIT`, and `LICENSE-APACHE`: license terms;
- `CHANGELOG.md`: release history.

## Start

Place `hydra` in a directory on your `PATH`, then verify it:

```bash
chmod +x hydra
./hydra --version
./hydra --help
```

On Windows, extract the ZIP to a stable directory on the Windows user `PATH`,
restart Git Bash, and run `hydra.exe --version` followed by `hydra --help`.
Git for Windows must also be installed and reachable from Git Bash.

The portable ZIP does not change your shell profile. Enable Tab completion by
adding this to Git Bash's `~/.bashrc`:

```bash
source "$(dirname "$(command -v hydra.exe)")/completions/hydra.bash"
```

If the packaged file is unavailable, generate the registration dynamically:

```bash
source <(hydra completions bash)
```

The optional Codex skill is installed explicitly after the executable is on
your `PATH`:

```bash
hydra skill install codex
```

Hydra preserves unknown or locally modified skill content. Inspect, update, or
remove only the copy managed by Hydra:

```bash
hydra skill status codex
hydra skill update codex
hydra skill remove codex
```

Full installation instructions, platform status, workflows, and safety
guidance are maintained in the
[English user guide](https://github.com/leonardoLoddo/hydra/blob/main/Docs/user/hydra-user-guide.md).

## License

Copyright 2026 Leonardo Loddo. Licensed, at your option, under the MIT License
or Apache License 2.0.

# Hydra release archive

Hydra is a Git-native workspace manager for isolated development Heads.
This archive contains a prebuilt `hydra` executable for the target named in
the archive filename; Rust is not required to run it.

## Contents

- `hydra`: the command-line executable;
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

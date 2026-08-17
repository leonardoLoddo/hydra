# Homebrew tap for Hydra

This tap distributes preview builds of
[Hydra](https://github.com/leonardoLoddo/hydra), a Git-native workspace manager
for isolated development Heads.

## Install

```bash
brew install leonardoLoddo/tap/hydra-heads
```

The Formula is called `hydra-heads` to avoid colliding with the unrelated
`hydra` package in Homebrew core. It installs the `hydra` executable.

After installation:

```bash
hydra --help
hydra skill install codex
```

The Codex skill is optional and is never installed by Homebrew itself.

## Update

```bash
brew update
brew upgrade leonardoLoddo/tap/hydra-heads
hydra skill status codex
```

## Uninstall

```bash
hydra skill remove codex
brew uninstall leonardoLoddo/tap/hydra-heads
```

Removing the Formula does not delete user-owned skill content.

<h1 align="center">
  <img src="assets/hydra-banner.png" alt="Hydra — One repository. Many isolated Heads.">
</h1>

[![CI](https://github.com/leonardoLoddo/hydra/actions/workflows/ci.yml/badge.svg)](https://github.com/leonardoLoddo/hydra/actions/workflows/ci.yml)
[![License: MIT or Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

Hydra is a Git-native workspace manager for isolated development **Heads**.
Each Head has its own working tree, index, and private branch, so humans and AI
agents can work in parallel without sharing uncommitted files.

> [!IMPORTANT]
> Hydra 1.x is the current SemVer compatibility line for the documented core.
> Distribution remains a public preview intended for a small group of testers
> while real projects complete field validation. Use it on repositories whose
> important work is already committed or backed up, and report unexpected Git
> or filesystem state before attempting manual repair.

Version `1.0.0` establishes the compatibility baseline for Hydra's documented
CLI, configuration, persisted state, and Head lifecycle. Future incompatible
changes to those contracts require another major version. The preview label
describes the current breadth of real-world qualification; it does not make
published compatibility guarantees disposable. See the
[latest release](https://github.com/leonardoLoddo/hydra/releases/latest) and
[changelog](CHANGELOG.md).

## Install

Install the current public preview from the dedicated Homebrew tap on macOS,
native Linux, or WSL 2:

```bash
brew install leonardoLoddo/tap/hydra-heads
```

The Formula is named `hydra-heads` because Homebrew already distributes an
unrelated package named `hydra`. The installed executable is still `hydra`.
The Formula also installs dynamic Bash, Zsh, and Fish completions in Homebrew's
standard directories; manual activation remains documented for shells that do
not load them automatically.

On native Windows x86-64, use the stable release links:

[Download Hydra for Windows](https://github.com/leonardoLoddo/hydra/releases/latest/download/hydra-windows-x86_64.zip)
· [SHA-256 checksum](https://github.com/leonardoLoddo/hydra/releases/latest/download/hydra-windows-x86_64.zip.sha256)

Extract the archive into a directory on the Windows `PATH`, then use
`hydra.exe` from Git Bash. The ZIP includes `completions/hydra.bash`; because
the archive is portable, activate that script explicitly from `~/.bashrc` as
described in the installation guide. The stable links always resolve through
the latest GitHub Release; the same release also contains the versioned
Windows archive and all macOS and Linux artifacts. Native Windows packaging is
available in published releases
starting with `v0.2.0`. Homebrew remains the installation channel for macOS,
Linux, and WSL 2.

To work from the repository instead, build from source with the pinned Rust
toolchain:

```bash
git clone https://github.com/leonardoLoddo/hydra.git
cd hydra
cargo install --path crates/hydra-cli --locked --force
```

Verify the executable you are using:

```bash
hydra --version
command -v hydra
```

## Quick start

Run these commands in an existing Git repository with at least one commit:

```bash
hydra init
hydra head create payment --from main --target main
hydra head path payment
hydra head status payment
```

Move your editor, terminal, or agent into the path printed by `head path`.
When the work is committed and ready to integrate, inspect it, return to the
parent project with the target branch checked out, and run:

```bash
hydra head close payment
```

Native close runs an ordinary `git merge` in the parent worktree and prints
Git's output directly. If Git reports conflicts, resolve and commit them there
while Hydra waits; Hydra then resumes protected Head removal automatically.
Running `git merge --abort` aborts the close and preserves the Head.

Use `hydra --help` and `hydra <command> --help` for the complete installed
syntax.

## Documentation

The complete [English user guide](Docs/user/hydra-user-guide.md) explains the
concepts, installation, Head lifecycle, configuration, storage and overlays,
Agent Skills, recovery, troubleshooting, and current CLI.

Focused pages:

- [Installation and updates](Docs/user/installation.md)
- [Core concepts](Docs/user/concepts.md)
- [Head workflows](Docs/user/head-workflows.md)
- [Configuration](Docs/user/configuration.md)
- [Storage and overlays](Docs/user/storage-and-overlays.md)
- [Windows copy-on-write setup](Docs/user/windows-copy-on-write.md)
- [WSL 2 copy-on-write setup](Docs/user/wsl-copy-on-write.md)
- [Agent Skills](Docs/user/agent-skills.md)
- [Recovery and troubleshooting](Docs/user/recovery-and-troubleshooting.md)
- [CLI reference](Docs/user/cli-reference.md)

The complete [Italian user guide](Docs/user/hydra-user-guide.it.md) is
maintained alongside the English documentation.

## Optional Agent Skill

Hydra ships one portable Agent Skill that teaches supported AI agents the safe
Head workflow. Homebrew never installs it silently. Choose the provider whose
personal skill directory you want Hydra to manage:

<p align="center">
  <img src="assets/hydra-codex-skill.png" alt="Codex prompt using the Hydra skill to create an isolated workflow for a payment feature">
</p>

```bash
hydra skill install codex
hydra skill install gemini
hydra skill install agy
hydra skill install antigravity
```

For unattended setup, make the choice explicit:

```bash
hydra skill install codex --yes
# or
hydra skill install codex --no
```

Manage only the copy installed by Hydra:

```bash
hydra skill status codex
hydra skill update codex
hydra skill remove codex

hydra skill status gemini
hydra skill update gemini
hydra skill remove gemini

hydra skill status agy
hydra skill update agy
hydra skill remove agy

hydra skill status antigravity
hydra skill update antigravity
hydra skill remove antigravity
```

Hydra preserves an unknown or locally modified skill instead of overwriting or
deleting it. Codex normally detects changes automatically. Gemini CLI can
rescan its skill directories with `/skills reload`. In Antigravity CLI, restart
the host if needed and use `/skills` to inspect discovery. In the Antigravity
app, restart if needed and check **Settings > Customizations > Skills**.

Gemini CLI also recognizes `$HOME/.agents/skills`, so it may discover the copy
installed for Codex without a second installation. Use `gemini` when you want
an independently managed copy in Gemini's native personal directory. AGY and
the Antigravity app use separate global locations.

## Update and uninstall

```bash
brew update
brew upgrade leonardoLoddo/tap/hydra-heads
```

After upgrading the binary, check each independently managed provider copy:

```bash
hydra skill status codex
hydra skill update codex
hydra skill status gemini
hydra skill update gemini
hydra skill status agy
hydra skill update agy
hydra skill status antigravity
hydra skill update antigravity
```

When upgrading from `0.2.x`, note the `1.0.0` breaking change: run
`hydra head close` from the canonical parent project with the recorded target
branch checked out. Git merges and any conflict resolution now happen in that
parent worktree.

Remove the binary and, only if desired, the skill:

```bash
hydra skill remove codex
hydra skill remove gemini
hydra skill remove agy
hydra skill remove antigravity
brew uninstall leonardoLoddo/tap/hydra-heads
```

Homebrew uninstall does not remove user-owned skill content.

## Supported preview platforms

Release automation builds native archives for:

- macOS on Apple Silicon (`aarch64-apple-darwin`);
- macOS on Intel (`x86_64-apple-darwin`);
- Linux ARM64 (`aarch64-unknown-linux-gnu`);
- Linux x86-64 (`x86_64-unknown-linux-gnu`);
- Windows x86-64 (`x86_64-pc-windows-msvc`).

Release automation is configured to verify Homebrew installation on both
macOS and Linux architectures before updating the tap. WSL 2 uses the Linux
Formula and remains preview evidence to exercise directly with a colleague;
native Windows is tested with Git for Windows and Git Bash. Windows artifacts
are published as both a versioned ZIP and the stable
`hydra-windows-x86_64.zip` download. WSL 1 is not supported.

The current Formula includes native archive metadata for Linux. WSL 2 uses
that Linux Formula, while direct end-to-end WSL evidence remains part of the
preview validation.

The preview build baseline is macOS 11 or newer, Linux distributions with
glibc 2.35 or newer (including Ubuntu 22.04 or newer), and Windows 11 x86-64.
On Windows, ReFS can provide block-clone COW while unsupported volumes safely
fall back to full copies; tracked and overlay symlinks remain unsupported.

## Development

Hydra requires the repository-pinned Rust toolchain. Before proposing a change,
run:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

See [AGENTS.md](AGENTS.md) and the
[documentation router](Docs/hydra-context-router.md) for the project contracts,
TDD workflow, and safety invariants.

Bug reports and preview feedback are welcome through the repository's
[issue templates](https://github.com/leonardoLoddo/hydra/issues/new/choose).
Read [CONTRIBUTING.md](CONTRIBUTING.md) before proposing a change and report
security-sensitive defects through the private process in
[SECURITY.md](SECURITY.md).

## License

Copyright 2026 Leonardo Loddo. Licensed, at your option, under the
[MIT License](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE).

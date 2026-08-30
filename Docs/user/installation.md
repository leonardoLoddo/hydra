# Installation and Updates

This page covers installing, verifying, updating, and removing Hydra. For the
first project workflow, continue with [Head workflows](head-workflows.md).

## Requirements and supported platforms

Hydra requires Git and currently targets:

- macOS 11 or newer on Apple Silicon or Intel;
- native Linux on ARM64 or x86-64 with glibc 2.35 or newer, including Ubuntu
  22.04 or newer;
- WSL 2 through the Linux Homebrew Formula;
- native Windows 11 x86-64 with Git for Windows, operated from Git Bash.

WSL 1 is not supported. WSL 2 uses the Linux artifact. Native Windows uses
`hydra.exe`; PowerShell and Command Prompt are useful for installation, but the
documented Hydra workflow and shell completions use Git Bash.

Installation support and storage capability are separate. The default WSL 2
root filesystem is commonly ext4 and may require Hydra's safe full-copy
fallback. For physical copy-on-write, place a new project and its sibling Heads
directory on the same reflink-capable Linux volume before `hydra init`; see
[WSL copy-on-write storage](storage-and-overlays.md#wsl-2-copy-on-write).

Release archives are native binaries. Installing a published archive or the
Homebrew Formula does not require a Rust toolchain.

## Install with Homebrew

Use the fully qualified Formula from Hydra's dedicated tap:

```bash
brew install leonardoLoddo/tap/hydra-heads
```

The Formula is named `hydra-heads` because other Homebrew packages already use
the name `hydra`. The executable installed on your `PATH` is still `hydra`.

The Formula provides native URLs for all supported macOS and Linux
architectures. On WSL, first confirm from PowerShell that the distribution
uses WSL 2:

```powershell
wsl -l -v
```

Use Homebrew's default Linux prefix on WSL. Do not move the installation to a
custom prefix merely to make Hydra visible; Homebrew bottles and environment
setup expect the standard Linuxbrew layout.

Homebrew prints Hydra's artwork, `hydra --help`, and the optional
`hydra skill install codex` command after installation. It never installs the
Codex skill silently.

## Verify the executable

After installation, run:

```bash
hydra --version
command -v hydra
hydra --help
```

If you have installed Hydra through both Homebrew and Cargo, inspect every
reachable binary:

```bash
which -a hydra
```

The first entry in `PATH` wins. A documentation or help mismatch often means
an older executable appears first.

## Install from source

Development builds use the toolchain pinned by the repository:

```bash
git clone https://github.com/leonardoLoddo/hydra.git
cd hydra
cargo install --path crates/hydra-cli --locked --force
```

This method requires Rust and Cargo. `--locked` uses the dependency versions
recorded by the repository; `--force` replaces a previous Cargo-installed copy
at the same destination. It does not remove a Homebrew-installed binary.

## Install a release archive manually

GitHub Releases publishes native archives and separate SHA-256 checksum files
for each supported target. Download the archive matching your operating system
and CPU from the
[latest release](https://github.com/leonardoLoddo/hydra/releases/latest),
verify it against the published checksum, and follow the packaged `README.md`.

The archive contains the `hydra` executable, licenses, changelog, terminal
artwork, and canonical Agent Skill. Place the executable in a directory on
your `PATH`; do not copy the skill into personal state manually. After the
binary is reachable, use `hydra skill install codex` for the protected optional
installation flow.

On native Windows, download the stable
[`hydra-windows-x86_64.zip`](https://github.com/leonardoLoddo/hydra/releases/latest/download/hydra-windows-x86_64.zip)
and its
[`SHA-256 checksum`](https://github.com/leonardoLoddo/hydra/releases/latest/download/hydra-windows-x86_64.zip.sha256).
The same release also retains the immutable
`hydra-<version>-x86_64-pc-windows-msvc.zip` archive. Verify the checksum and
extract the ZIP to a stable directory. Add that directory to the Windows user
`PATH`, restart Git Bash, and verify:

```bash
hydra.exe --version
command -v hydra
hydra --help
```

Git Bash resolves `hydra` to `hydra.exe`, so all normal examples continue to
use the extensionless command. Git for Windows must remain reachable on
`PATH`, because Hydra invokes `git` directly.

Native Windows packaging is available starting with `v0.2.0`. The stable
download resolves through the latest GitHub Release, while the versioned
archive remains available for reproducible downloads.

## Update Hydra

Update the Homebrew Formula with:

```bash
brew update
brew upgrade leonardoLoddo/tap/hydra-heads
hydra --version
```

Each optional provider copy has a separate lifecycle and is never replaced by
a Homebrew upgrade. Inspect and update every installed copy explicitly:

```bash
hydra skill status codex
hydra skill update codex
hydra skill status gemini
hydra skill update gemini
hydra skill status agy
hydra skill update agy
```

Hydra asks for confirmation with a default-negative choice before changing an
installed skill. See [Agent Skills](agent-skills.md) for provider destinations,
automation, and ownership rules.

## Remove Hydra

Remove only the binary with:

```bash
brew uninstall leonardoLoddo/tap/hydra-heads
```

If you also want to remove the unmodified skill managed by Hydra, do that
explicitly before or after uninstalling the Formula:

```bash
hydra skill remove codex
hydra skill remove gemini
hydra skill remove agy
brew uninstall leonardoLoddo/tap/hydra-heads
```

Homebrew uninstall does not delete user-owned or locally modified skill
content. Removing the executable also does not remove project `.hydra.json`
files, local Heads, private branches, or Hydra's project state.

## Enable shell completion

Hydra generates completion registration for Bash, Zsh, and Fish. Source it at
shell startup so it uses the currently installed executable.

For Bash, add this to `~/.bashrc`:

```bash
source <(hydra completions bash)
```

For Zsh, add this to `~/.zshrc`:

```zsh
source <(hydra completions zsh)
```

For Fish, add this to `~/.config/fish/config.fish`:

```fish
hydra completions fish | source
```

Completion covers commands and options. It also proposes existing local Head
names for `head status`, `head path`, `head open`, `head close`, and
`head remove`. It deliberately does not propose existing names for
`head create`.

Outside a readable Hydra project, dynamic Head completion returns no
candidates and no visible error. Reload the shell configuration after a Hydra
upgrade.

## Next step

Read [Core concepts](concepts.md), then initialize a disposable or backed-up
repository using [Head workflows](head-workflows.md).

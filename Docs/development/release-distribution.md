# Release and Distribution

## Purpose

This document defines the release and distribution contract for Hydra.
It owns repository naming, release automation, Homebrew publication, packaged
assets, and the boundary between a non-interactive package manager and Hydra's
interactive skill installer. It does not make an unavailable installer or
package user-visible before the corresponding code, tests, artifacts, and
release workflow exist.

## Current Status

Public preview `v0.1.1` is available from GitHub Releases and the
`leonardoLoddo/homebrew-tap` tap. The current Formula contains immutable native
archive URLs for both macOS and Linux on ARM64 and x86-64. WSL 2 consumes the
Linux Formula. Public `v0.1.1` predates the native Windows artifacts. Current
release automation adds native Windows x86-64 to the same release transaction
as a checksummed ZIP containing `hydra.exe`; that channel becomes public with
the first subsequent release and is exercised with Git for Windows and Git Bash.
Direct end-to-end WSL evidence remains part of preview validation.
The intended audience remains a small group of colleagues who can exercise
preview releases and report platform, installation, upgrade, and workflow
defects before broader promotion.

The source repository remains `leonardoLoddo/hydra`. The executable remains
named `hydra`.

## License

Hydra is available under `MIT OR Apache-2.0`, at the recipient's option.
`LICENSE-MIT` contains the MIT terms and the copyright notice for Leonardo
Loddo. `LICENSE-APACHE` contains the unmodified Apache License 2.0 terms, while
the root `LICENSE` file states the choice without creating another set of
terms. Workspace and package Cargo metadata must use the same SPDX expression.

Release archives and source distributions include all three files. Generated
Homebrew metadata represents the choice as:

```ruby
license any_of: ["MIT", "Apache-2.0"]
```

The repository README may use presentation assets that do not belong in binary
archives. Each release archive therefore exposes the compact
`packaging/release/README.md` as its root `README.md` and excludes the repository
banner. The compact README must remain sufficient to identify the archive,
start the binary, manage the optional skill, find the canonical documentation,
and understand the license without carrying multi-megabyte visual assets.

## Repository and Homebrew Topology

The source repository and the Homebrew tap have separate responsibilities:

- `leonardoLoddo/hydra` owns source, documentation, release tags,
  checksums, and GitHub release artifacts;
- the dedicated `leonardoLoddo/homebrew-tap` repository owns generated Formula
  metadata;
- the Formula is named `hydra-heads` and installs the `hydra` executable,
  because `homebrew/core` already owns an unrelated `hydra` Formula.

The intended preview command is therefore:

```bash
brew install leonardoLoddo/tap/hydra-heads
```

The fully qualified Formula name is required in maintained instructions so the
user explicitly trusts the intended third-party package. The Formula must
declare its conflict with another package that installs the same `hydra`
executable when Homebrew cannot safely link both.

## Installation and Skill Boundary

Homebrew installation, upgrade, test, and bundle workflows must remain
non-interactive. A Formula must not prompt, write into a user's Codex home, or
silently install the Hydra skill. It installs versioned Hydra artifacts and may
print concise caveats that direct the user to the optional skill step. The
Formula caveats must first render the complete versioned `hydra-art.txt` asset,
including its `HYDRA` wordmark, and then end with copyable orientation and skill
commands:

```text
[complete contents of hydra-art.txt]

Get started:
  hydra --help

Optional Codex skill:
  hydra skill install codex
```

Homebrew therefore finishes successfully even in unattended automation, while
an interactive user immediately sees how to continue the installation
experience. Running `hydra skill install codex` remains an explicit opt-in and
must not be a Formula `post_install` action.

The Formula renders the artwork verbatim and without ANSI control sequences.
Release validation must compare the displayed block with the packaged asset and
prove that the artwork appears before both command suggestions. The Formula
must not maintain a second hand-edited copy that can drift from
`hydra-art.txt`.

The public skill-management hierarchy is:

```text
hydra skill install codex
hydra skill status codex
hydra skill update codex
hydra skill remove codex
```

Hydra's first-party `hydra skill install codex` command owns the desired
interactive installation experience. When run on an interactive terminal it
must:

1. explain that Codex skill installation is optional;
2. show the resolved Codex destination and ask for confirmation with a
   default-negative choice;
3. default to not installing the skill when input is empty, unavailable, or
   interrupted;
4. show and validate the destination before copying anything;
5. report whether the skill was installed, skipped, or refused safely.

Codex is the only initial provider. The provider remains explicit in the
command so future adapters do not change the meaning of an existing invocation.
Automation must have explicit non-interactive choices equivalent to installing
the Codex skill or skipping it. It must never infer consent from the presence
of Codex.

## Codex Skill Packaging

Every release derives the installable skill from the canonical
`skills/hydra/` directory. The packaged copy records enough release provenance
to distinguish the same Hydra version from an unknown or locally modified
installation.

Codex currently discovers personal skills below `$HOME/.agents/skills`, so the
Codex adapter installs Hydra at `$HOME/.agents/skills/hydra`. This host contract
must be revalidated against the
[official Codex skill documentation](https://developers.openai.com/codex/skills)
before changing the adapter; `CODEX_HOME/skills` is not the current personal
skill location.

Skill installation must:

- respect the documented Codex personal-skill location;
- avoid overwriting an existing skill of unknown origin or with local changes;
- publish a complete directory atomically where the platform permits;
- provide inspect, update, and removal paths, not installation alone;
- state when Codex must be restarted or refreshed;
- keep `SKILL.md` and `agents/openai.yaml` identical to the verified release
  artifact rather than maintaining an installer-specific fork.

The Codex adapter may add a `.hydra-skill.json` provenance manifest beside the
portable artifact. It records the provider, Hydra version, and SHA-256 digest
of every canonical installed file. `update` and `remove` must validate that
manifest and the exact installed tree before mutation; an unknown origin,
missing manifest, symlink, extra entry, or changed digest is treated as a local
modification and preserved. The manifest is generated by the installer and is
not a second copy of the skill instructions.

## Version and Release Convention

Hydra continues to use the Conventional Commits rules in
[`commit-conventions.md`](commit-conventions.md). Normal commits determine the
next semantic version:

- `fix` produces a patch candidate;
- `feat` produces a minor candidate;
- any allowed type with `!` or a `BREAKING CHANGE:` footer produces a major
  candidate;
- documentation, tests, refactors, build, CI, and chores do not independently
  request a version bump unless they intentionally carry a breaking-change
  marker.

There is no manually invented `release` commit type. Release automation opens
a reviewable release pull request that updates all linked Cargo package
versions, `Cargo.lock`, and `CHANGELOG.md`. Its generated commit uses the
existing convention, for example:

```text
chore(release): release v0.1.0
```

Merging that pull request creates the immutable `vX.Y.Z` tag and GitHub
Release. Publication is triggered by the created release or tag, not merely by
a commit whose message resembles a release, so partial or forged publication
states cannot update Homebrew.

Release Please creates the reviewable release pull request and a draft GitHub
Release because it consumes the Conventional Commits convention already used
by Hydra. Hydra uses a reviewed repository-owned workflow rather than
`cargo-dist`: the custom Formula caveats, portable skill payload, ordered draft
publication, native runner matrix, and separately scoped tap update require a
small explicit transaction that remains easier to audit directly. Every
third-party action is pinned to an immutable commit SHA.

Hydra keeps Cargo's inherited `version.workspace = true` declarations as the
canonical package-version relationship. Release Please uses its `simple`
strategy as the single release coordinator because its `cargo-workspace`
plugin cannot parse inherited package versions. Targeted TOML updaters change
`workspace.package.version` and both Hydra entries in `Cargo.lock` in the same
release pull request. The checked-in `version.txt` is only the coordinator's
required release marker; release-tooling validation requires it to equal the
version reported for every workspace package by locked Cargo metadata. The
configured initial release version is a bootstrap floor: it must equal the
marker before the first release so a repository with no prior tag starts at the
intended preview version rather than the strategy's stable-release default.
After that release it remains unchanged while `version.txt` advances, and
release-tooling validation rejects only an invalid value or one newer than the
current marker.

The release matrix builds native archives for macOS Apple Silicon, macOS Intel,
Linux ARM64, Linux x86-64, and native Windows x86-64. Unix targets use
`.tar.gz`; Windows publishes both the immutable versioned ZIP and a byte-for-byte
identical `hydra-windows-x86_64.zip` alias containing `hydra.exe`. The stable
alias and its checksum make the README's latest-release link independent of the
version number, while the versioned archive remains available for reproducible
downloads. The preview baseline
is macOS 11 or newer, glibc 2.35 or newer, and Windows 11 with Git for Windows,
built on native macOS 15, Ubuntu 22.04, and Windows 2025 runners.
Generated Homebrew metadata selects the matching immutable archive on all four
targets. Before updating the tap, release automation performs Formula audit,
installation, caveat verification, skill lifecycle, and uninstall on both
architectures of macOS and Linux. WSL 2 consumes the Linux Formula but still
requires direct preview evidence. Native Windows uses its release ZIP rather
than Homebrew; WSL 1 is not supported.

## Publication Transaction

A release is publishable only after this ordered evidence exists:

1. mandatory Rust quality gates and platform integration tests pass;
2. the release pull request contains coherent Cargo versions and changelog;
3. a version tag and draft GitHub Release are created;
4. release binaries and the canonical skill artifact are built for the
   declared platforms;
5. every artifact has a published SHA-256 checksum;
6. the draft receives the verified artifacts and is published;
7. Formula audit, clean install, test, skill lifecycle, artwork ordering, and
   uninstall pass on each published Homebrew architecture;
8. only then is the tap Formula updated to the immutable release artifacts.

The tap update uses a repository-scoped credential that can write only the tap
where technically possible. The source workflow must not hold a broad personal
token when a narrower GitHub App or fine-grained token can satisfy the same
operation.

Release publication jobs that intentionally run without a source checkout must
identify `leonardoLoddo/hydra` explicitly when invoking GitHub CLI. Publication
must not depend on discovering repository identity from the runner's current
directory.

Homebrew smoke tests register the generated Formula in a temporary tap and
audit it by its tap-qualified name. They must not pass a Formula path to
`brew audit`, because supported Homebrew versions require a Formula name.

The Formula derives its version from the immutable release URLs and must not
repeat it with an explicit `version` declaration. Release archives are always
built from the requested tag. Formula generation and smoke testing use the
selected workflow revision: on a tag-triggered run this is the tag commit,
while a manual recovery run may use corrected tooling from the selected branch
without moving the tag or changing the tagged release sources.

## Preview Validation

During the colleague preview, release evidence and direct tester feedback must
verify at least:

- installation on a clean supported macOS machine without a preinstalled Rust
  toolchain;
- installation on clean supported native Linux runners without a preinstalled
  Rust toolchain;
- extraction and execution of the native Windows ZIP through Git Bash without
  a preinstalled Rust toolchain;
- `hydra --version` reports the tagged version;
- Git dependency discovery and a disposable `hydra init` workflow succeed;
- accepting the Codex prompt installs the exact packaged skill;
- declining or running without a terminal installs no skill;
- an existing or locally modified Codex skill is preserved;
- Homebrew upgrade updates the executable and offers an explicit skill update
  path without silently replacing it;
- Homebrew uninstall does not silently delete user-owned or modified skill
  content;
- the complete packaged artwork appears at the end of Homebrew installation,
  before both command suggestions, without corrupting logs;
- install, update, and removal instructions in the README and maintained
  English and Italian user documentation match the released artifacts.

Preview limitations must be stated in the GitHub Release and README. WSL or a
platform not directly exercised by the release matrix is not described as
supported merely because a binary can start there.

## Remaining Preview Evidence

The following external evidence is still required before broader promotion:

- verify a clean colleague installation without Rust;
- exercise the complete Homebrew workflow directly on WSL 2 before promoting
  it beyond preview support;
- verify the real Homebrew upgrade path from `v0.1.0` to `v0.1.1` on a clean
  preview machine.

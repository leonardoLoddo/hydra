# Release and Distribution

## Purpose

This document defines the planned release and distribution contract for Hydra.
It owns repository naming, release automation, Homebrew publication, packaged
assets, and the boundary between a non-interactive package manager and Hydra's
interactive skill installer. It does not make an unavailable installer or
package user-visible before the corresponding code, tests, artifacts, and
release workflow exist.

## Current Status

Public distribution is **planned and not yet available**. The first intended
audience is a small group of colleagues who can exercise preview releases and
report platform, installation, upgrade, and workflow defects before broader
promotion.

The final source-repository name is fixed as `leonardoLoddo/hydra-heads`.
Moving the current `leonardoLoddo/hydra` repository to that name is an
operational release-readiness step, not an open naming decision. The executable
remains named `hydra`.

## Repository and Homebrew Topology

The source repository and the Homebrew tap have separate responsibilities:

- `leonardoLoddo/hydra-heads` owns source, documentation, release tags,
  checksums, and GitHub release artifacts;
- a dedicated tap repository, preferably `leonardoLoddo/homebrew-tap`, owns
  generated Formula metadata;
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
Formula caveats must end with copyable orientation and skill commands:

```text
Get started:
  hydra --help

Optional Codex skill:
  hydra skill install codex
```

Homebrew therefore finishes successfully even in unattended automation, while
an interactive user immediately sees how to continue the installation
experience. Running `hydra skill install codex` remains an explicit opt-in and
must not be a Formula `post_install` action.

The public skill-management hierarchy is planned as:

```text
hydra skill install codex
hydra skill status codex
hydra skill update codex
hydra skill remove codex
```

Hydra's first-party `hydra skill install codex` command owns the desired
interactive installation experience. When run on an interactive terminal it
must:

1. render the Hydra artwork from the versioned `hydra-art.txt` asset;
2. render a terminal wordmark spelling `HYDRA` below the artwork;
3. explain that Codex skill installation is optional;
4. show the resolved Codex destination and ask for confirmation with a
   default-negative choice;
5. default to not installing the skill when input is empty, unavailable, or
   interrupted;
6. show and validate the destination before copying anything;
7. report whether the skill was installed, skipped, or refused safely.

The artwork currently uses Unicode block and Braille characters rather than
strict seven-bit ASCII. Onboarding must detect a non-interactive, unsuitable,
or narrow terminal and use a compact text fallback instead of emitting broken
layout or control sequences.

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

Skill installation must:

- respect `CODEX_HOME` and the documented Codex personal-skill location;
- avoid overwriting an existing skill of unknown origin or with local changes;
- publish a complete directory atomically where the platform permits;
- provide inspect, update, and removal paths, not installation alone;
- state when Codex must be restarted or refreshed;
- keep `SKILL.md` and `agents/openai.yaml` identical to the verified release
  artifact rather than maintaining an installer-specific fork.

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

Release Please is the preferred candidate for the release pull request because
it consumes the Conventional Commits convention already used by Hydra.
`cargo-dist` is the preferred candidate for building checksummed Rust release
archives and publishing the third-party Homebrew Formula. Both remain subject
to a focused evaluation before adoption; generated workflows and actions must
be version-pinned and reviewed like production supply-chain code.

## Publication Transaction

A release is publishable only after this ordered evidence exists:

1. mandatory Rust quality gates and platform integration tests pass;
2. the release pull request contains coherent Cargo versions and changelog;
3. an immutable version tag and GitHub Release are created;
4. release binaries and the canonical skill artifact are built for the
   declared platforms;
5. every artifact has a published SHA-256 checksum;
6. a clean install and smoke test pass for each published Homebrew target;
7. only then is the tap Formula updated to the new immutable artifacts;
8. the Formula audit, install, test, upgrade, and uninstall paths pass.

The tap update uses a repository-scoped credential that can write only the tap
where technically possible. The source workflow must not hold a broad personal
token when a narrower GitHub App or fine-grained token can satisfy the same
operation.

## Preview Acceptance

Before inviting colleagues, a preview release must verify at least:

- installation on a clean supported macOS machine without a preinstalled Rust
  toolchain;
- `hydra --version` reports the tagged version;
- Git dependency discovery and a disposable `hydra init` workflow succeed;
- accepting the Codex prompt installs the exact packaged skill;
- declining or running without a terminal installs no skill;
- an existing or locally modified Codex skill is preserved;
- Homebrew upgrade updates the executable and offers an explicit skill update
  path without silently replacing it;
- Homebrew uninstall does not silently delete user-owned or modified skill
  content;
- artwork and the compact fallback both render without corrupting logs;
- install, update, and removal instructions in the README and Italian user
  guide match the released artifacts.

Preview limitations must be stated in the GitHub Release and README. WSL or a
platform not directly exercised by the release matrix is not described as
supported merely because a binary can start there.

## Release Readiness Gaps

The following decisions or artifacts are still required before the first
preview release:

- choose and add an explicit open-source license;
- create the project README with install, opt-in skill, update, uninstall,
  support, and preview-status guidance;
- rename the GitHub source repository and update every canonical link;
- create and secure the Homebrew tap repository;
- implement the `hydra skill` lifecycle and interactive installation behavior
  with TDD;
- select and pin the release automation after validating its generated diff;
- add the release and Homebrew smoke-test matrix;
- define the supported release architectures and minimum platform versions.

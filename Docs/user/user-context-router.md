# User Documentation Context Router

## Purpose

This router owns Hydra's end-user documentation: installation, supported
command workflows, configuration guidance, customization, safety expectations,
and troubleshooting for behavior available in the current release.

The granular English guide is the primary public entry point. The maintained
Italian guide provides the same complete operating boundary in one localized
document. User documentation explains released or currently implemented
behavior and does not redefine product or architecture contracts.

---

## Routes

| Document | Consult when the task involves | Nature of the document |
|---|---|---|
| [`hydra-user-guide.md`](hydra-user-guide.md) | Any end-user documentation change or a user-visible behavior that must remain discoverable and navigable | Primary English guide and documentation index. It owns the quick start, reading paths, global safety summary, and current capability overview. |
| [`installation.md`](installation.md) | Installation, supported platforms, Homebrew, source builds, upgrades, uninstall, executable discovery, or shell registration | English installation and lifecycle guide for the distributed executable. |
| [`concepts.md`](concepts.md) | Explaining Heads, parent-project normalization, refs, isolation, storage, overlays, shared configuration, or local state to users | English conceptual model and lifecycle overview. |
| [`head-workflows.md`](head-workflows.md) | Initialization, creation, naming, inspection, opening, integration, closing, removal, or operating Hydra from a Head | English task-oriented guide for the complete Head lifecycle. |
| [`configuration.md`](configuration.md) | `.hydra.json`, directory policy, branch prefix, storage policy, overlay policy, open adapters, close adapters, placeholders, or local-state boundaries | English configuration and trusted-command guide. |
| [`storage-and-overlays.md`](storage-and-overlays.md) | Copy-on-write, full-copy fallback, storage diagnostics, overlay rules, ignored files, symlinks, prompts, submodules, or secret-handling boundaries | English storage and overlay guide. |
| [`windows-copy-on-write.md`](windows-copy-on-write.md) | Native Windows copy-on-write, ReFS, Dev Drive setup, Windows full-copy diagnostics, same-volume placement, or Windows storage troubleshooting | Focused English setup guide linked by initialization, storage diagnosis, and Head creation when native Windows falls back to full copy. |
| [`wsl-copy-on-write.md`](wsl-copy-on-write.md) | WSL 2 copy-on-write, Linux reflink volumes, XFS setup boundaries, WSL full-copy diagnostics, same-volume placement, or WSL storage troubleshooting | Focused English setup guide linked by initialization, storage diagnosis, and Head creation when WSL falls back to full copy. |
| [`codex-skill.md`](codex-skill.md) | Installing, invoking, updating, removing, packaging, or explaining Hydra's optional Codex Agent Skill | English human guide for the skill lifecycle and operating boundary. |
| [`recovery-and-troubleshooting.md`](recovery-and-troubleshooting.md) | Repair planning, deterministic recovery, report-only inconsistencies, locks, missing inventory, incomplete operations, or actionable errors | English recovery and troubleshooting guide. |
| [`cli-reference.md`](cli-reference.md) | A command, argument, option, output-composition rule, or supported shell or provider | Compact English reference for the current command surface. |
| [`hydra-user-guide.it.md`](hydra-user-guide.it.md) | Any user-visible command, option, default, output, workflow, configuration field, customization, safety constraint, supported platform, or troubleshooting case | Maintained complete Italian guide. It must stay behaviorally aligned with the routed English guide. |

---

## Consultation Rules

- Consult `hydra-user-guide.md` and every affected focused English page for
  each externally observable CLI or configuration change.
- Consult the Italian guide for the same change and update both languages in
  the same task when a command, option, default, output, workflow step,
  validation rule, supported customization, or user recovery action changes.
- Update the English index when navigation, onboarding order, global safety
  guidance, or current capability overview changes.
- Describe only behavior verified in the current implementation. Do not place
  roadmap items, planned commands, speculative syntax, or future capabilities
  in user documentation; those belong in routed product documentation.
- Keep examples copyable and consistent with `hydra --help` and the
  implementation.
- Combine this route with the product router when scope or intended behavior
  changes, and with the architecture router when implementation state or known
  limitations determine what users can safely do.

Routes are cumulative. User documentation translates authoritative contracts
into operating guidance; it does not override them.

---

## Domain Maintenance

Add another user document only when it has a stable responsibility that makes
the existing English guide materially easier to navigate. Keep the index and
Italian guide aligned with the resulting coverage.

When adding, renaming, moving, or removing a document:

1. update the **Routes** table;
2. update consultation rules when ownership changes;
3. update the parent router at
   [`../hydra-context-router.md`](../hydra-context-router.md);
4. verify every relative link and command example.

Do not store implementation notes, task logs, or speculative command syntax in
this domain.

# Hydra AI-Agent Skill Maintenance Standard

**Status:** current
**Scope:** source, packaging, synchronization, and verification of `skills/hydra/`
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read this leaf when a task changes the installable Hydra Agent Skill or any
workflow, command, configuration, safety, troubleshooting, or recovery rule
that the skill projects to agents.

## Purpose

This document owns the packaging, synchronization, and verification rules for
Hydra's installable AI-agent skill. It does not redefine CLI, product, Git,
filesystem, configuration, or recovery behavior. The skill projects those
canonical contracts into concise operational instructions for an agent.

The maintained artifact lives at `skills/hydra/` and ships with Hydra.

---

## Artifact Contract

The skill contains only files needed by an agent at runtime:

```text
skills/hydra/
├── SKILL.md
├── agents/
│   └── openai.yaml
└── references/
    ├── arena.md
    ├── augury.md
    └── gauntlet.md
```

- `SKILL.md` owns triggering metadata and vendor-neutral operating guidance.
- `agents/openai.yaml` owns Codex-facing display metadata and a default prompt.
- `references/` owns detailed, independently selected Hydra Art guidance;
  `SKILL.md` owns the routing condition for each reference.
- Maintainer documentation, installation guides, test logs, and changelogs do
  not belong inside the skill directory.
- Add scripts, references, or assets only when a concrete runtime need justifies
  progressive loading or reusable execution. Do not duplicate canonical project
  documentation as a bundled reference.

Keep the skill concise. It must tell an agent how to operate Hydra safely, not
teach general Git usage or preserve implementation history.

The canonical `SKILL.md` is vendor-neutral and follows the portable
[Agent Skills specification](https://agentskills.io/specification).
Provider-specific metadata such as
`agents/openai.yaml` may improve discovery or presentation for one host, but it
must not own workflow or safety behavior that other compatible agents need.

---

## Distribution ownership

Load [release-distribution.md](release-distribution.md) when changing provider
adapters, personal installation destinations, manifests, opt-in installation,
release archives, or host qualification. That leaf canonically owns distribution
policy; this leaf owns the vendor-neutral instructional artifact and its
synchronization with Hydra behavior. Distribution adapters MUST project the same
artifact rather than fork instructions for each vendor.

## Authoritative Inputs

Separate approved product and workflow authority from implementation evidence.
Derive the skill through these relationships:

1. routed product documentation defines scope and safety invariants;
2. the maintained granular English guide and Italian guide define supported
   workflows available to users;
3. current CLI help, code, and tests prove which syntax and behavior are
   implemented;
4. the skill converts those contracts into agent actions and stop conditions.

When these sources disagree, stop and report the conflict. Do not update the
skill to hide a documentation or implementation mismatch.

The skill must remain as vendor-neutral as practical. Codex-specific packaging
belongs in `agents/openai.yaml`; operating instructions in `SKILL.md` should be
portable to other agents unless a platform capability requires otherwise.

---

## Mandatory Synchronization

Assess skill impact for every non-trivial task. Update `skills/hydra/` in the
same change whenever an agent-visible contract changes, including:

- commands, subcommands, arguments, options, defaults, exit behavior, or
  relevant output;
- project initialization, Head creation, inspection, opening, closing,
  removal, repair, or handoff workflows;
- configuration fields, placeholders, installation steps, or supported
  customization;
- Git, filesystem, ownership, isolation, destructive-action, confirmation,
  rollback, or recovery rules;
- troubleshooting instructions or conditions that require an agent to stop
  and ask for direction;
- the skill's trigger description, packaging layout, or installation path.

An impact assessment without a skill edit is sufficient only when the changed
contract cannot affect what an agent executes, decides, validates, or reports.
State that conclusion in the final report. Never make a meaningless wording or
timestamp change merely to touch the skill.

When an agent parses inspection data instead of presenting it to a person, the
skill MUST prefer the supported `--json` mode on `status`, `head list`, `head
status`, `head path`, `doctor storage`, and `skill status`. It MUST NOT parse
human-readable summaries when a versioned JSON contract exists, infer JSON on
mutating commands, or treat JSON selection as authorization for an action.

When `SKILL.md` changes, inspect `agents/openai.yaml` in the same task and
regenerate it if the display name, short description, or default prompt is no
longer aligned.

---

## Safety Projection

The skill must preserve these operating properties:

- inspect the installed CLI and repository state before mutation;
- choose Head name, source, and target explicitly;
- move task work into the path returned by Hydra;
- keep edits and authorized commits on the private Head branch;
- distinguish the Head work directory from a separate control worktree when
  the versioned configuration is unavailable in the Head;
- leave completed Heads available for review by default;
- require explicit authorization before integration, forced removal, or any
  action that can discard files or update a target ref;
- use Hydra's protected lifecycle commands instead of manual metadata,
  filesystem, or destructive worktree operations;
- stop with evidence when ownership, policy, paths, locks, Git state, or
  recovery behavior is ambiguous.

Never weaken one of these rules merely to simplify an example or reduce the
number of steps.

---

## Verification

For every skill change:

1. run the skill-structure validator against `skills/hydra/`;
2. inspect `agents/openai.yaml` for consistency with `SKILL.md`;
3. run `git diff --check` and verify every changed documentation link;
4. compare every documented command and option with the installed or newly
   built `hydra --help` hierarchy;
5. confirm that the English and Italian user documentation describe only
   currently supported workflows and agree with product documentation.

When the skill contains references, install it into a temporary provider home
and verify that every routed file is present, checksummed in the provenance
manifest, protected from local-modification overwrite or removal, and included
in Unix and Windows release-package tests.

When operating workflow or safety guidance changes, also exercise the affected
path in a newly created temporary Git repository. Verify both the intended
action and the nearest protected failure, and never use the Hydra source
repository or an existing user Head as a destructive test target.

Rust TDD and Cargo quality gates remain mandatory when production behavior
changes. A skill-only or documentation-only change does not require artificial
Rust tests; validate structure, links, consistency, and applicable disposable
workflow behavior directly.

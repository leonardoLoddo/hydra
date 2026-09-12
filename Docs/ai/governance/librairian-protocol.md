# Repository LibrAIrian Protocol

**Protocol version:** 1.0.0
**Status:** current
**Canonical owner:** [ROUTER.md](ROUTER.md)
**Scope:** repository-local AI-facing project knowledge

## Consult when

Read this document for protocol adoption, routing or knowledge changes,
documentation authoring, audits, upgrades, and fallback operation when the
`librairian` skill is unavailable.

## Operational entry points

- Preferred procedure: `.agents/skills/librairian/SKILL.md`
- Project knowledge entry point: `Docs/ai/ROUTER.md`
- Portable fallback: this document
- Canonical upstream: `https://github.com/leonardoLoddo/librairian`

The checked-in skill MUST remain a complete, validated copy of the user-approved
LibrAIrian Protocol distribution. Upgrade the skill, fallback protocol, protocol
version, and affected routing mechanics together.

## Operating rules

1. Open `Docs/ai/ROUTER.md` before classifying a task against repository
   concerns.
2. Follow every applicable route. Routing is cumulative.
3. Read required knowledge before making project-specific decisions.
4. Resolve explicit inherited-default dependencies and apply only documented
   exceptions or overrides.
5. Combine selected knowledge with targeted implementation evidence.
6. Separate normative authority from descriptive evidence and report
   conflicts.
7. Routers select; leaves explain.
8. Every active leaf has one canonical owner.
9. Persist only verified or explicitly uncertain knowledge that is reusable,
   non-obvious, durable, and worth its cost.
10. Update affected knowledge and routes in the same change as implementation.
11. Perform the knowledge-impact check before completing every implementation
    task.

## Authority and evidence

Normative intent comes from the current authorized request, approved
requirements and decisions, repository-local policy within scope, then generic
guidance.

Runtime, persisted state, tests, code, schema, configuration, dependencies, and
version-control state are descriptive evidence. When intent and reality
disagree, report the conflict. Do not silently convert either side into the
other.

## Status, scope, and inherited rules

Use `current`, `proposed`, `disputed`, and `historical` as status values. Current
leaves govern only their declared scope. Proposed or historical material MUST NOT
enter ordinary implementation context unless its explicit consultation condition
holds. Historical knowledge names a successor when retained; Git history normally
preserves superseded text without a duplicate active archive.

`MUST` and `MUST NOT` are mandatory. `SHOULD` is the default unless a documented
project-specific reason justifies an exception. `MAY` is optional.

At one authority level, apply the most specific documented override or exception,
then the nearest inherited default, then broader defaults. An override explicitly
replaces a rule or extends it with constraints. Defaults MUST state applicability
and MUST NOT be inferred from repetition. Unrelated conflicting defaults are
ambiguous: report them and stop the affected decision. Do not invent an exception
because implementation differs or a case appears unusual.

Resolve explicit dependencies once and keep inheritance shallow. Cross-links for
related reading do not automatically inherit rules. Routers select knowledge and
record canonical ownership; they do not own shared defaults themselves.

## Authoring standard

AI-facing knowledge uses controlled technical English unless Hydra explicitly
adopts another language for a routed document.

Each leaf states status, scope, canonical owner, consultation trigger,
applicable rules and invariants, exceptions and failure behavior, change
impact, evidence references, and observable verification when relevant. Omit
empty sections.

Use canonical domain terms and exact code identifiers. Each durable rule has
one canonical owner. Link instead of copying rules.

Prefer structural compression over cryptic wording. Keep shared defaults in
leaves with explicit applicability. Dependent leaves MUST name and link the
canonical default, and selected context MUST include it. Exceptions identify
the default and scope; overrides explicitly replace or extend it. Never infer
exceptions from implementation. Keep enough safety-critical context locally.

Create a leaf only when it has a distinct consultation trigger, coherent
scope, independent lifecycle, and future value greater than its context and
maintenance cost. Split by semantic trigger, not line count. Merge material
that is always selected together.

Remove decorative prose, task history, vague verification, synonym drift,
cheap code narration, generic framework behavior, secrets, personal data, and
speculation presented as fact.

Token reduction MUST preserve scope, authority, conditions, exceptions,
failure behavior, safety, authorization, and verification.

## Knowledge-impact check

After implementation verification, answer all five questions:

1. Did behavior, architecture, terminology, workflow, dependency, invariant, or
   operational procedure change?
2. Did routed knowledge or a local annotation become false, incomplete, misplaced,
   or redundant?
3. Was verified reusable non-obvious knowledge discovered?
4. Should knowledge be added, updated, moved, consolidated, split, demoted, or removed?
5. Did routing, priority, ownership, or cross-context composition change?

Update the canonical owner in the same change; remove stale copies and update
all affected owning and ancestor routers. Recheck dependent defaults and local
annotations. `No knowledge update required` is valid only with a concrete reason.
Routine maintenance is scoped to affected knowledge, not a repository-wide audit.

A local `AI-KNOWLEDGE` annotation is optional only for a verified, component-local
semantic invariant that a test, type, assertion, or ordinary comment cannot express
more safely. Place it adjacent to the smallest scope, include `Invariant`, and add
rationale, change impact, or evidence only when useful. Never encode authorship,
sessions, cheap source structure, or speculative rules in annotations.

## Validation

For a knowledge change, audit, installation, or fallback check, load
[knowledge-validation.md](knowledge-validation.md). It owns runnable structural
checks, routing and inheritance cases, evidence review, and operational verification.
Run only the checks applicable to the actual change. Structural success does not
prove semantic truth, freshness, route completeness, or information value.

## Procedure without the skill

For adoption, inventory current knowledge and relevant historical sources, classify
status, route valuable content before rewriting it, and consolidate only with clear
ownership. For an audit, diagnose first; repair only within authorized scope.
For a protocol version upgrade, read an explicit approved migration before changing
version or reinterpreting policy. If no migration exists, request a decision.
A same-version user-approved skill refresh still compares changed mechanics and
synchronizes this fallback and affected routes; do not invent a new version.

Before accepting a leaf, verify explicit scope and exclusions, authority, terminology,
conditions, exceptions, inherited defaults, evidence, and observable completion.
Reject task residue, duplicate authority, generic guidance, and cheap code narration.
Every leaf needs an independent consultation trigger; split by semantic selection,
not a line quota. Preserve safety-critical context when removing repeated copies.

Report procedure, scope, protocol version before and after, knowledge changes,
evidence, conflicts, actual structural/routing/semantic/operational checks, and
remaining uncertainty. Do not claim whole-system correctness from sampled claims.

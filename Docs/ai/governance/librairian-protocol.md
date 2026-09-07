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

The checked-in skill MUST remain a complete, validated copy of one approved
LibrAIrian Protocol release. Upgrade the skill, fallback protocol, protocol
version, and affected routing mechanics together.

## Operating rules

1. Open `Docs/ai/ROUTER.md` before classifying a task against repository
   concerns.
2. Follow every applicable route. Routing is cumulative.
3. Read required knowledge before making project-specific decisions.
4. Combine selected knowledge with targeted implementation evidence.
5. Separate normative authority from descriptive evidence and report
   conflicts.
6. Routers select; leaves explain.
7. Every active leaf has one canonical owner.
8. Persist only verified or explicitly uncertain knowledge that is reusable,
   non-obvious, durable, and worth its cost.
9. Update affected knowledge and routes in the same change as implementation.
10. Perform the knowledge-impact check before completing every implementation
    task.

## Authority and evidence

Normative intent comes from the current authorized request, approved
requirements and decisions, repository-local policy within scope, then generic
guidance.

Runtime, persisted state, tests, code, schema, configuration, dependencies, and
version-control state are descriptive evidence. When intent and reality
disagree, report the conflict. Do not silently convert either side into the
other.

## Authoring standard

AI-facing knowledge uses controlled technical English unless Hydra explicitly
adopts another language for a routed document.

Each leaf states status, scope, canonical owner, consultation trigger,
applicable rules and invariants, exceptions and failure behavior, change
impact, evidence references, and observable verification when relevant. Omit
empty sections.

Use canonical domain terms and exact code identifiers. Each durable rule has
one canonical owner. Link instead of copying rules.

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

Ask whether the implementation changed behavior, architecture, terminology,
workflow, dependency, invariant, operational procedure, routing, priority,
canonical ownership, or reusable verified knowledge.

Update, move, consolidate, split, demote, or remove affected knowledge. `No
knowledge update required` is valid only with a concrete reason.

## Validation

Validate affected files, links, reachability, canonical ownership, route
behavior, and changed claims against current evidence. Structural checks do
not prove semantic truth, freshness, or route completeness.

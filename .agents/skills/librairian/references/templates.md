# LibrAIrian Templates

Adapt paths, language, domains, evidence, and validation commands to the repository. Remove unused optional sections. Do not leave placeholders in installed artifacts.

## Instruction-file block

Merge this block into the applicable `AGENTS.md` or equivalent file. Preserve unrelated instructions.

```markdown
## LibrAIrian Protocol

LibrAIrian Protocol maintenance is required for every implementation task.

Before making project-specific decisions, open `Docs/ai/ROUTER.md` and follow every applicable route. Routing is cumulative. Combine selected knowledge with targeted inspection of code, tests, configuration, schema, version-control state, and runtime evidence.

Use the `librairian` skill when available. Otherwise follow the repository-local protocol routed from `Docs/ai/ROUTER.md`.

Before completing an implementation task, perform the knowledge-impact check. Update affected canonical knowledge and routes in the same change, then validate them. `No knowledge update required` is valid only with a concrete reason.

Report material conflicts between normative authority and current implementation. Do not encode unresolved assumptions as current policy.
```

Do not place domain routes or leaf paths in the instruction file.

## Macro-router

```markdown
# Project Knowledge Router

**LibrAIrian Protocol:** 1.0.0

## Purpose

This is the single entry point for repository-local project knowledge.

## Routing algorithm

1. Read this router before classifying the task against repository concerns.
2. Identify every concern touched by the task.
3. Follow every matching route; routes are cumulative.
4. Process each router once.
5. Read required knowledge before recommended or reference material.
6. Combine selected knowledge with targeted implementation evidence.
7. Report material conflicts or unresolved ambiguity.

## Controlled concern vocabulary

| Concern | Meaning | Excludes |
|---|---|---|
| <canonical concern> | <semantic boundary> | <plausible near-match> |

## Routes

| Conditions | Priority | Read | Skip when |
|---|---|---|---|
| any: <task behavior or concern> | required | [<domain>/ROUTER.md](<domain>/ROUTER.md) | <negative condition> |

## Cumulative examples

- <multi-domain task> selects <route A> and <route B> because <reason>.

## No exact match

Use the closest route for investigation. Do not create current knowledge until scope, evidence, and canonical ownership are established.

## Ownership and maintenance

- Routers select; leaves explain.
- Every leaf has one canonical owning router.
- Cross-context routes link without copying authority.
- Update affected routes when an artifact is created, moved, renamed, split, merged, demoted, or removed.

## Verification

Run: `<repository-specific command or documented manual check>`
```

## Governance router

```markdown
# Project Knowledge Governance Router

## Purpose and scope

Route tasks that install, maintain, audit, or upgrade LibrAIrian Protocol and its documentation.

## Selection rules

| Conditions | Priority | Read | Skip when |
|---|---|---|---|
| any: protocol adoption, router change, knowledge maintenance, documentation authoring or review, audit, fallback operation | required | [librairian-protocol.md](librairian-protocol.md) | ordinary implementation after all project routes are selected |

## Cross-context composition

Add every domain router whose knowledge is being created or changed.

## Maintenance

This router canonically owns `librairian-protocol.md`. Update the macro-router only when the governance boundary or top-level path changes.
```

## Repository-local fallback protocol

Install at `Docs/ai/governance/librairian-protocol.md`. This compact fallback owns the repository operating contract when the skill is unavailable.

```markdown
# Repository LibrAIrian Protocol

**Protocol version:** 1.0.0
**Status:** current
**Canonical owner:** `Docs/ai/governance/ROUTER.md`
**Scope:** repository-local AI-facing project knowledge

## Consult when

Read this document for protocol adoption, routing or knowledge changes, documentation authoring, audits, upgrades, and fallback operation when the `librairian` skill is unavailable.

## Operating rules

1. Open `Docs/ai/ROUTER.md` before classifying a task against repository concerns.
2. Follow every applicable route. Routing is cumulative.
3. Read required knowledge before making project-specific decisions.
4. Combine selected knowledge with targeted implementation evidence.
5. Separate normative authority from descriptive evidence and report conflicts.
6. Routers select; leaves explain.
7. Every active leaf has one canonical owner.
8. Persist only verified or explicitly uncertain knowledge that is reusable, non-obvious, durable, and worth its cost.
9. Update affected knowledge and routes in the same change as implementation.
10. Perform the knowledge-impact check before completing every implementation task.

## Authority and evidence

Normative intent comes from the current authorized request, approved requirements and decisions, repository-local policy within scope, then generic guidance.

Runtime, persisted state, tests, code, schema, configuration, dependencies, and version-control state are descriptive evidence. When intent and reality disagree, report the conflict. Do not silently convert either side into the other.

## Authoring standard

AI-facing knowledge uses controlled technical English unless the repository explicitly chooses another language.

Each leaf states status, scope, canonical owner, consultation trigger, applicable rules and invariants, exceptions and failure behavior, change impact, evidence references, and observable verification when relevant. Omit empty sections.

Use canonical domain terms and exact code identifiers. Each durable rule has one canonical owner. Link instead of copying rules.

Create a leaf only when it has a distinct consultation trigger, coherent scope, independent lifecycle, and future value greater than its context and maintenance cost. Split by semantic trigger, not line count. Merge material always selected together.

Remove decorative prose, task history, vague verification, synonym drift, code narration, generic framework behavior, secrets, personal data, and speculation presented as fact.

Token reduction MUST preserve scope, authority, conditions, exceptions, failure behavior, safety, authorization, and verification.

## Knowledge-impact check

Ask whether the implementation changed behavior, architecture, terminology, workflow, dependency, invariant, operational procedure, routing, priority, canonical ownership, or reusable verified knowledge.

Update, move, consolidate, split, demote, or remove affected knowledge. `No knowledge update required` is valid only with a concrete reason.

## Validation

Validate affected files, links, reachability, canonical ownership, route behavior, and changed claims against current evidence. Structural checks do not prove semantic truth, freshness, or route completeness.
```

## Domain router

```markdown
# <Domain> Project Knowledge Router

## Purpose and scope

<Semantic boundary owned by this router and adjacent concerns it excludes.>

## Selection rules

| Conditions | Priority | Read | Skip when |
|---|---|---|---|
| any: <positive behaviors or concerns> | <required, recommended, or reference> | [<leaf>.md](<leaf>.md) | <near-miss exclusion> |

## Cross-context composition

- Add [../<other-domain>/ROUTER.md](../<other-domain>/ROUTER.md) when <condition>.

## Routing examples

- Positive: <task> selects <leaf> because <reason>.
- Negative: <task> excludes <leaf> because <reason>.
- Cumulative: <task> also selects <other route> because <reason>.

## Ownership and maintenance

This router canonically owns: <leaf list>. Update it when owned knowledge is created, moved, renamed, split, merged, demoted, or removed.
```

## Knowledge leaf

```markdown
# <Knowledge unit>

**Status:** current | proposed | disputed | historical
**Scope:** <components, versions, environments, or domain boundary>
**Canonical owner:** [ROUTER.md](ROUTER.md)
**Last verified:** <YYYY-MM-DD, only when actually verified>
**Review when:** <events likely to invalidate this knowledge>

## Consult when

<Positive task behaviors and important exclusions.>

## Canonical terms

<Only terms needed to avoid ambiguity.>

## Rules and invariants

<Normative rules, constraints, and prohibitions.>

## Decisions and rationale

<Non-obvious approved intent. Omit if unnecessary.>

## Exceptions and failure behavior

<Conditions, alternate behavior, rejection, retry, or fallback. Omit if irrelevant.>

## Change impact

<Coupled behavior, consumers, risks, and verification radius.>

## Evidence

- Implementation: `<repository-relative path or stable symbol>`
- Verification: `<test, command, schema, or observable runtime check>`

## Verification

<Observable checks required after relevant changes.>

## Related knowledge

<Links without copied authority.>
```

## Routing case

Use this shape for difficult or high-risk routing boundaries. Store cases where repository conventions can exercise them.

```yaml
name: <short case name>
task: <representative request>
expected_concerns:
  - <concern>
expected_routers:
  - Docs/ai/<domain>/ROUTER.md
expected_required_leaves:
  - Docs/ai/<domain>/<leaf>.md
excluded_leaves:
  - Docs/ai/<other>/<unrelated>.md
rationale: <why the selection is complete and minimal>
```

Do not require a routing case for every obvious leaf. Add cases where ambiguity, cumulative composition, authorization, safety, or repeated failure makes them valuable.

## Local knowledge annotation

Use the language's native comment or docblock style:

```text
AI-KNOWLEDGE
Invariant: <verified component-local semantic rule>
Reason: <non-obvious rationale>
Change impact: <coupled behavior to inspect>
Evidence: <stable test, document, or code symbol>
```

Only `Invariant` is required. Prefer a test, type, assertion, or ordinary comment when it can express and enforce the fact more safely.

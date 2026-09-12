# LibrAIrian Protocol

**Protocol version:** 1.0.0

**Status:** normative

## Purpose

LibrAIrian Protocol defines how a repository exposes the smallest complete set of durable project knowledge required for an AI-assisted task.

The protocol governs discovery, routing, authority, ownership, lifecycle, validation, and fallback operation. It does not replace implementation inspection, tests, runtime observation, user documentation, ADRs, issues, pull requests, or changelogs.

## Canonical terms

Use these terms consistently:

| Term | Meaning |
|---|---|
| LibrAIrian | The overall system and public identity |
| LibrAIrian Protocol | This versioned operational specification |
| selective project knowledge routing | The architectural pattern at LibrAIrian's core |
| project knowledge | Durable repository-owned knowledge that changes future decisions or verification |
| instruction file | `AGENTS.md` or equivalent file that activates LibrAIrian |
| knowledge root | AI-facing knowledge directory, normally `Docs/ai/` |
| macro-router | `Docs/ai/ROUTER.md`, the single knowledge entry point |
| concern | A behavior, responsibility, or risk recognizable in a task |
| domain router | A router that selects knowledge inside a semantic boundary |
| knowledge leaf | One coherent independently selectable knowledge unit |
| structural compression | Removal of repeated rule copies through explicit scoped defaults and deviations without shortening their semantic content |
| default | Stable shared rule inherited by declared narrower scopes |
| applicability | Explicit scope in which a default, exception, or override governs behavior |
| exception | Intentional scoped deviation from a named or structurally adjacent default |
| override | Local rule that explicitly replaces an inherited rule or extends it with additional constraints |
| consultation trigger | Positive condition requiring a route or leaf |
| skip condition | A plausible near-match that does not require the route or leaf |
| canonical owner | The only router responsible for registering a leaf |
| cumulative routing | Union of every applicable route rather than first-match classification |
| knowledge-impact check | Required completion review of implementation effects on project knowledge |
| local knowledge annotation | Optional `AI-KNOWLEDGE` block for a verified component-local semantic constraint |
| normative authority | Source that defines what ought to be true |
| descriptive evidence | Source that shows what is currently true |

Do not introduce alternate operational names for LibrAIrian or LibrAIrian Protocol. Historical design names are provenance, not protocol vocabulary.

## Normative language

`MUST` and `MUST NOT` define mandatory behavior. `SHOULD` defines the default unless a documented project-specific reason justifies an exception. `MAY` defines optional behavior.

## Authority and evidence

Normative authority and descriptive evidence answer different questions and MUST NOT be flattened into one source-of-truth list.

### Normative authority

In descending task-local authority:

1. the current authorized user request and explicit constraints;
2. approved requirements, decisions, policies, and contracts within their declared scope;
3. repository-local project knowledge and conventions within their declared scope;
4. generic ecosystem guidance.

A later or more specific normative decision supersedes an older or broader decision only when the relationship is explicit.

### Descriptive evidence

Runtime behavior, persisted state, focused tests, reproducible commands, source code, schema, configuration, dependencies, and version-control state describe current reality.

Evidence strength depends on relevance, recency, reproducibility, and environment. A passing test proves only the behavior it covers.

### Conflict rule

When normative intent and descriptive reality disagree, the agent MUST identify the conflicting sources and affected scope. It MUST NOT silently redefine intent from implementation or describe desired behavior as already implemented.

Unresolved claims MUST be marked `proposed`, `disputed`, `historical`, or unknown rather than current fact.

## Required repository architecture

An adopted repository contains:

1. a compact LibrAIrian trigger in its applicable instruction file;
2. one macro-router at `Docs/ai/ROUTER.md`;
3. a governance route containing the repository-local LibrAIrian Protocol fallback;
4. only domain routers justified by distinct consultation triggers;
5. knowledge leaves with one canonical owner each.

```text
instruction file
      |
      v
Docs/ai/ROUTER.md
      |
 cumulative routes
      |
domain routers
      |
knowledge leaves
      |
targeted implementation evidence
```

The skill is the preferred procedure. The repository-local protocol is the portable fallback. The repository MUST remain operable without a machine-local skill path.

## Routing contract

The agent MUST:

1. read applicable instruction files;
2. open the macro-router before classifying the task against repository concerns;
3. identify every concern touched by the task;
4. follow every matching route;
5. traverse each router at most once;
6. collect the union of selected leaves;
7. resolve every explicit inherited-default dependency of those leaves;
8. read `required` knowledge before project-specific decisions;
9. load `recommended` and `reference` knowledge only when its conditions hold;
10. inspect relevant implementation evidence;
11. report material conflicts or unresolved ambiguity.

Routing is cumulative. No router MAY stop traversal merely because one route matched.

Routers SHOULD use explicit `all`, `any`, and `unless` conditions when prose would be ambiguous. Difficult boundaries SHOULD include positive, negative, and cumulative examples.

Navigation from the entry point is structurally verifiable. Semantic task classification remains agent judgment. The protocol MUST NOT be described as fully deterministic.

## Context priorities

| Priority | Contract |
|---|---|
| required | Necessary for correctness, safety, authorization, or an applicable contract; MUST NOT be dropped for a numeric budget |
| recommended | Reduces investigation or risk; read when its trigger holds or required knowledge is insufficient |
| reference | Supporting detail loaded on demand |

Context efficiency is measured across selection and use, not by individual file length. Prefer precise routes, skip conditions, canonical references, stable terminology, and progressive disclosure.

Compression MUST preserve scope, authority, conditions, exceptions, failure behavior, safety, authorization, and verification.

## Router contract

A router:

- owns a semantic selection boundary;
- defines recognizable concerns and conditions;
- points to routers or leaves using repository-relative paths;
- states when each target is required and when it can be skipped;
- supports cumulative cross-domain composition;
- records canonical ownership;
- remains smaller and less detailed than the knowledge it selects.

A router MUST NOT become a domain manual, duplicate leaf rules, or classify only by source directory.

## Knowledge leaf contract

A leaf MUST:

- govern one coherent knowledge unit;
- have a distinct consultation trigger;
- have exactly one canonical owning router;
- declare current status and scope;
- distinguish normative rules from evidence, history, and proposals;
- state relevant invariants, prohibitions, exceptions, and failure behavior;
- provide stable implementation or evidence references when useful;
- define observable verification;
- link related knowledge without copying its rules.

A leaf MUST NOT narrate source code, preserve task history, duplicate generic framework guidance, contain secrets or personal data, or encode speculation as fact.

Detailed language, quality, and granularity rules are canonical in [authoring-standard.md](authoring-standard.md).

## Structural compression

LibrAIrian prefers structural compression over semantic compression. Authors SHOULD remove repeated rule copies by declaring stable shared behavior once. They MUST NOT make individual rules cryptic merely to reduce tokens.

A default:

- MUST state or inherit clear applicability;
- SHOULD govern multiple narrower scopes without modification;
- MUST remain a leaf rule, not routing detail;
- MUST NOT be inferred from repetition, examples, or current implementation.

A leaf that depends on a default owned elsewhere MUST identify the default and link its canonical source. The selected context MUST include that source. Repositories MAY mirror these relationships in maintained structured metadata, but optional metadata MUST NOT replace an understandable normative declaration.

An exception MUST identify, by explicit reference or unambiguous adjacent structure, the default it modifies. It MUST state its applicability and contain enough rule context to be interpreted without guessing.

An override MUST state whether it:

- **replaces** the inherited rule within its applicability; or
- **extends** the inherited rule with additional constraints.

Within the same authority level, apply the most specific applicable documented override or exception before the nearest inherited default, then broader defaults. If applicable defaults conflict and no explicit override relationship resolves them, the knowledge is ambiguous. The agent MUST surface the conflict instead of choosing silently.

Agents MUST NOT invent an exception because a case appears unusual. Descriptive evidence that contradicts an inherited rule is handled through the authority and evidence conflict rule; it does not silently become an exception.

Inheritance SHOULD remain shallow and visible. If exceptions are numerous, unstable, or harder to understand than the inherited cases, authors SHOULD narrow the default or keep the rules local.

Security, authorization, persistence boundaries, financial calculations, destructive operations, and externally visible behavior MAY use defaults. The local leaf MUST restate enough of the safety-critical rule and name its canonical source when inheritance alone would create material interpretation risk.

## Canonical ownership

Each active leaf has one canonical owner. Other routers MAY select that leaf as cross-context without claiming ownership.

When a leaf is created, moved, renamed, split, merged, deprecated, or removed, the owning router and every affected ancestor MUST change in the same intervention.

Move a rule without leaving two normative copies. Remove obsolete active content; version control preserves history.

## Knowledge admission rule

Persist a candidate only when it is:

- verified or explicitly uncertain;
- reusable beyond the current task;
- non-obvious or expensive to rediscover;
- relevant to future decisions, actions, or verification;
- assigned to one canonical owner;
- worth more than its expected retrieval and maintenance cost.

Do not persist transient debugging state, session narration, agent identity, authorship logs, generic advice, or cheaply derivable imports, exports, callers, inheritance, and file inventories.

## Local knowledge annotations

A local `AI-KNOWLEDGE` annotation MAY be used only when a verified semantic fact:

- applies primarily to one component;
- materially constrains how that component may change;
- is likely to be missed during a local edit;
- is too narrow for a routed leaf;
- cannot be expressed more safely through a test, type, assertion, or ordinary explanatory comment.

The annotation MUST be adjacent to the smallest governed scope. `Invariant` is required. `Reason`, `Change impact`, and `Evidence` are included only when useful.

Annotations MUST NOT record sessions, agent identity, authorship, speculation, task history, imports, exports, callers, or other cheaply derivable structure.

## Knowledge lifecycle

Every implementation task MUST perform the knowledge-impact check after implementation verification.

Ask:

1. Did behavior, architecture, terminology, workflow, dependency, invariant, or operational procedure change?
2. Did routed knowledge or a local annotation become false, incomplete, misplaced, or redundant?
3. Was verified reusable non-obvious knowledge discovered?
4. Should knowledge be added, updated, moved, consolidated, split, demoted, or removed?
5. Did routing, priority, ownership, or cross-context composition change?

When knowledge changes, update the canonical owner in the same intervention and validate the affected context.

`No knowledge update required` is valid only with a concrete reason.

Agents MAY maintain approved current knowledge. They MUST NOT self-approve new product policy or resolve material normative ambiguity by fiat.

## Status vocabulary

Use only:

- `current`: applicable and authoritative within scope;
- `proposed`: future or unapproved guidance;
- `disputed`: conflicting claims awaiting resolution;
- `historical`: retained as evidence and not applicable to new work.

Historical material MUST identify its current successor when one exists and MUST NOT be loaded as active guidance by default.

## Validation contract

Validation is proportional to the change:

- **structural:** files, links, reachability, ownership, cycles, paths, names, version, and inherited-default references;
- **routing:** representative tasks select all and only expected context, including required inherited defaults;
- **semantic:** changed claims match approved intent and current evidence, and defaults, applicability, exceptions, and override effects are unambiguous;
- **operational:** the instruction trigger, skill path, and repository-local fallback work in the real flow.

Structural validation MUST NOT be reported as proof of semantic truth, freshness, routing completeness, or information value.

## Definition of done

An adopted repository conforms when:

- its instruction file activates LibrAIrian and points to one macro-router;
- the macro-router records protocol version 1.0.0;
- every active leaf is reachable and canonically owned;
- route composition is cumulative;
- project knowledge follows the authoring standard;
- implementation tasks perform the knowledge-impact check;
- applicable validation passes;
- unresolved contradictions are visible rather than encoded as fact.

## Explicit exclusions

LibrAIrian Protocol 1.0.0 does not define code-analysis systems, generators, hooks, CI products, or external platforms. A repository MAY implement its own validation, but that tooling is not part of this protocol.

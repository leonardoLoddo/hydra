# LibrAIrian Authoring Standard

**Status:** normative

Use this standard whenever AI-facing project knowledge is created, rewritten, translated, compressed, split, merged, or substantially reviewed.

## Objective

An AI-facing document MUST let an agent determine:

1. what the document governs;
2. when it must be loaded and when it can be skipped;
3. which statements are normative;
4. which terms and invariants control decisions;
5. where current evidence can be inspected;
6. how completion can be verified.

Optimize total selected context and interpretation cost. Do not optimize sentence length in isolation.

## Language policy

AI-facing project knowledge MUST use controlled technical English by default.

A repository MAY choose another language through an explicit project decision. The chosen language MUST then be applied consistently within each routed document.

Human documentation MAY use the language appropriate for its readers. Human documentation MUST NOT become active agent context merely because it explains the same product area.

### Controlled technical English

Use:

- short declarative sentences;
- explicit subjects and actions;
- one requirement per sentence when practical;
- stable canonical terminology;
- literal wording;
- conditions before the actions they control;
- explicit prohibitions;
- concrete verbs such as `load`, `validate`, `persist`, `reject`, `retry`, `render`, and `verify`.

Preferred forms:

```text
<Actor> MUST <action>.
<Actor> MUST NOT <action>.
If <condition>, <actor> MUST <action>.
Use <component> for <purpose>.
Load <document> when <trigger>.
Skip <document> when <exclusion>.
Verify <observable result> with <method>.
```

Avoid:

- unclear pronouns;
- idioms, metaphors, humor, and rhetorical questions;
- decorative prose;
- long subordinate-clause chains;
- vague modifiers such as `usually`, `properly`, `appropriate`, and `as needed`;
- modal verbs with unclear strength in normative rules;
- stylistic synonyms for one domain concept;
- slash constructions when order or alternatives matter;
- invented abbreviations or private shorthand.

Do not translate code identifiers. Use exact class, method, route, event, permission, configuration, and state names.

## Normative precision

Use `MUST`, `MUST NOT`, `SHOULD`, and `MAY` with the meanings defined in `protocol.md`.

Each normative rule SHOULD make its actor, action, scope, condition, and exception explicit.

Bad:

> Older records should be handled properly when needed.

Good:

> If `schema_version < 3`, `LegacyOrderNormalizer` MUST run before validation.

Do not infer product policy from current implementation. Describe implementation as evidence unless the behavior is approved or enforced as a rule.

## Canonical terminology

Each domain SHOULD maintain a small vocabulary when multiple names could create ambiguity.

```markdown
| Canonical term | Meaning | Avoid for this meaning |
|---|---|---|
| import job | One asynchronous feed-processing execution | run, process |
| source record | One record read from the external feed | row, item |
```

The avoid column applies only to the same concept. It does not ban unrelated valid uses of those words.

Before introducing a term:

1. search documentation and code for the concept and likely synonyms;
2. choose the term already canonical in approved policy or code;
3. define any necessary mapping between display labels and internal identifiers;
4. replace accidental synonym drift inside the owned scope.

## Knowledge admission gate

Before writing a durable rule, verify all applicable conditions:

- **evidence:** the claim is verified or uncertainty is explicit;
- **reuse:** it will affect more than the current task;
- **decision value:** it changes inspection, choice, action, exception handling, or verification;
- **non-obviousness:** code or generic framework knowledge cannot recover it cheaply and reliably;
- **durability:** it is expected to remain useful long enough to justify maintenance;
- **ownership:** one existing or proposed router can own it;
- **economics:** its future value exceeds context and maintenance cost.

If these conditions fail, keep the information in code, tests, an issue, a plan, a pull request, a changelog, or temporary working notes as appropriate.

## Granularity contract

A leaf MUST represent one coherent knowledge unit, not one source file and not an arbitrary topic label.

Create a new leaf only when:

1. it has a distinct consultation trigger;
2. its scope can be stated without opening unrelated documents;
3. it can evolve independently;
4. it has a canonical owner;
5. selecting it separately saves context or prevents a material error.

### Split a leaf when

- sections have different consultation triggers;
- sections have different authority or status;
- sections change on different events;
- one section is required while another is only reference material;
- the document contains independent knowledge that is frequently loaded separately.

### Do not split when

- the only reason is line count;
- each fragment needs the others to be understood;
- all fragments are always selected together;
- fragmentation would repeat scope, terminology, or rationale;
- the resulting files would contain only one trivial rule each.

### Merge leaves when

- they are always selected together;
- their scopes and review triggers are the same;
- separation creates duplicate context;
- neither has independent ownership or lifecycle value.

There is no mandatory line or token limit. Size is a symptom to investigate, not a semantic boundary.

## Router authoring

A router answers: “Which knowledge must be loaded for this task?”

It SHOULD contain:

- purpose and semantic boundary;
- controlled concern vocabulary where useful;
- explicit positive triggers;
- relevant near-miss exclusions;
- cumulative cross-context rules;
- target paths;
- priority;
- difficult positive, negative, or cumulative examples;
- ownership and maintenance rule.

It MUST NOT contain:

- detailed domain implementation;
- copied leaf rules;
- long rationale already owned elsewhere;
- a flat file inventory without selection semantics;
- first-match logic.

A slightly longer routing condition is justified when it prevents an irrelevant leaf from entering context.

## Leaf structure

Every current leaf MUST expose status, scope, canonical owner, and consultation trigger. Other sections are included only when they contain useful information.

Recommended order:

1. title;
2. status, scope, canonical owner, and verification freshness when meaningful;
3. purpose and consultation trigger;
4. canonical concepts and terminology;
5. invariants, constraints, and prohibitions;
6. decisions and non-obvious rationale;
7. exceptions and failure behavior;
8. change impact;
9. implementation or evidence references;
10. verification;
11. related knowledge.

Omit empty sections. Do not force one template onto content that needs fewer sections.

## Status and freshness

Use only the protocol status vocabulary: `current`, `proposed`, `disputed`, and `historical`.

Add `last_verified`, evidence, owner team, or review triggers only when they will be maintained or validated. Metadata that nobody uses creates false confidence.

Use a review trigger when freshness depends on recognizable changes:

- provider API version changes;
- schema or permission changes;
- migration of an owning component;
- runtime ownership changes;
- replacement of a canonical test or contract.

Do not invent owners, dates, evidence, or validation results.

## Rules and rationale

Document rationale when it:

- prevents reversal of a non-obvious decision;
- explains a constraint not visible in code;
- helps decide an edge case;
- identifies the risk prevented by a prohibition.

Keep rationale adjacent to its rule. Do not repeat general motivation in every leaf.

Examples MUST clarify a boundary and MUST NOT introduce undeclared requirements.

## Exceptions and failure behavior

Document:

- the condition that activates the exception;
- the altered action or invariant;
- rejected or retryable behavior;
- fallback ownership;
- user-visible or operational consequences when relevant;
- verification of the exceptional path.

Do not use `handle errors appropriately` or equivalent vague language.

## Evidence and references

Prefer repository-relative paths and stable symbols:

```markdown
- Implementation: `app/Billing/InvoiceTotalCalculator.php`
- Contract: `InvoiceTotalCalculator::calculate()`
- Verification: `tests/Feature/Billing/InvoiceTotalTest.php`
```

Avoid durable line numbers unless maintained by tooling. Lines drift; paths and symbols are more stable.

Evidence supports a claim but does not automatically make the claim normative.

## Verification writing

Verification MUST name an observable result and a method.

Bad:

> Ensure everything still works.

Good:

> Run `InvoiceLifecycleTest` and verify invoice creation, refund authorization, and tax-rounding cases pass.

For UI behavior, include rendered browser verification when unit tests cannot prove the contract. For runtime or persisted-state rules, identify the relevant environment and observation without exposing sensitive data.

## Redundancy control

Before adding or repeating a rule:

1. search for the concept and its synonyms;
2. identify the canonical owner;
3. update the owner if the rule belongs there;
4. link from other required contexts;
5. remove or demote stale duplicates.

A router MAY contain a one-line description used for selection. That summary MUST NOT become a second detailed source.

Duplicate a safety-critical rule only when local visibility is necessary. The duplicate MUST name the canonical source and remain a deliberate summary rather than an independent authority.

## Human and agent documentation

Human documentation explains product use, onboarding, history, tutorials, or broader rationale. Agent project knowledge constrains engineering decisions.

Human documentation SHOULD link to canonical technical rules instead of redefining them. If it is not intended for agent context, keep it outside active routes or label it accordingly.

## Local annotation quality gate

Before adding `AI-KNOWLEDGE`, verify:

- the invariant is verified and component-local;
- missing it would create a material implementation risk;
- a test, type, assertion, or ordinary comment cannot express it more safely;
- the annotation does not duplicate a routed rule without naming its canonical source;
- every included field changes a future decision or verification;
- the annotation is adjacent to the smallest governed scope.

Do not use local annotations to make the source tree narrate itself.

## Authoring quality gate

Before completion, verify:

### Scope and authority

- scope and exclusions are explicit;
- consultation trigger is recognizable;
- normative rules are distinct from evidence and proposals;
- each durable rule has one canonical owner;
- conflicts are resolved or marked.

### Semantics and language

- the repository language policy is followed;
- canonical terms match domain and code;
- normative strength is unambiguous;
- conditions, exceptions, failure behavior, and prohibitions are explicit;
- vague language and synonym drift are removed.

### Granularity and context

- the leaf has an independent selection boundary;
- always-co-selected material is not fragmented without reason;
- repeated rules use canonical references;
- routers contain selection logic, not leaf detail;
- required content remains available under context constraints.

### Evidence and verification

- important paths, symbols, commands, and tests exist or uncertainty is stated;
- verification is observable and proportionate;
- freshness metadata is truthful;
- historical material is excluded from current guidance.

### Value

- the document changes a future decision, action, inspection, exception, or verification;
- expected reuse justifies context and maintenance cost;
- decorative prose, task residue, and cheap code narration are absent.

## Final heuristic

Keep a sentence when it changes at least one of:

- context selection;
- a decision;
- an allowed or prohibited action;
- exception handling;
- evidence inspection;
- completion verification.

Otherwise remove it, move it to human documentation, or replace it with a canonical reference.

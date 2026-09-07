# LibrAIrian Procedures

Read this reference for the procedure selected in `SKILL.md`. All procedures preserve unrelated instructions, user work, approved policy, and authorization boundaries.

## Shared preparation

Before any procedure:

1. Resolve the repository root.
2. Find every applicable instruction file from the root to the task scope.
3. Inspect current version-control state without altering unrelated changes.
4. Detect an existing `Docs/ai/ROUTER.md` and installed LibrAIrian Protocol version.
5. Identify the requested scope and whether the user authorized diagnosis, repair, or both.

Do not turn a documentation task into code, runtime, or external-system changes unless they are required and authorized by the request.

## Initialize

Use for a new or nearly empty repository.

### Discover

1. Inspect the product boundary, stack, source layout, tests, schema, configuration, CI, deployment, and current instructions.
2. Record candidate knowledge internally.
3. Classify each candidate as explicit rule, enforced rule, repeated intentional convention, local example, proposal, or unknown.
4. Identify only semantic areas with a current independent consultation trigger.
5. Record unresolved decisions without converting them into current policy.

### Design

1. Choose the knowledge root. Use `Docs/ai/` unless the repository has a stronger established convention.
2. Define the first controlled concern vocabulary from actual tasks and behavior.
3. Create the minimum domain set. Do not prebuild empty future branches.
4. Assign one canonical owner to each initial leaf.
5. Define project-specific structural validation that can actually be run.

### Install

1. Create `Docs/ai/ROUTER.md`.
2. Create `Docs/ai/governance/ROUTER.md`.
3. Install `Docs/ai/governance/librairian-protocol.md` from the compact fallback template.
4. Create only justified domain routers and leaves.
5. Add the LibrAIrian block to the applicable instruction file after the fallback route resolves.
6. Preserve all unrelated instruction-file content.

### Verify

1. Confirm every concrete link resolves.
2. Confirm every active artifact is reachable from the macro-router.
3. Confirm each leaf has one canonical owner.
4. Simulate one single-domain task, one cumulative task, one near-miss, and one knowledge-maintenance task.
5. Sample important claims against current evidence.
6. Verify that an agent without the skill can reach the repository-local fallback.

### Report

Report installed version, created artifacts, ownership map, evidence used, validation run, unresolved decisions, and deliberately deferred domains.

## Adopt

Use for a mature repository with existing documentation and conventions.

### Inventory

1. Inspect instructions, active documentation, ADRs, plans, historical material, code, tests, schema, configuration, and relevant runtime entry points.
2. Search for repeated rules and conflicting terminology.
3. Classify existing material as `current`, `proposed`, `disputed`, `historical`, human documentation, or task record.
4. Identify repeated investigation costs and high-risk knowledge gaps.

### Map

For each valuable artifact:

- identify its consultation trigger;
- identify what it excludes;
- decide whether it is a router, leaf, human document, or historical evidence;
- choose one canonical owner;
- record duplicates to consolidate;
- record claims that require verification.

### Install incrementally

1. Create macro-router and governance fallback.
2. Route valuable existing documents before rewriting them.
3. Introduce only domains with distinct triggers.
4. Consolidate duplicate rules without losing project-specific exceptions.
5. Mark historical and proposed material explicitly.
6. Add the instruction-file trigger only after the fallback path works.

Do not attempt an exhaustive rewrite during initial adoption.

### Verify and report

Use real repository tasks for positive, negative, cumulative, ambiguous, and maintenance cases. Report coverage achieved and knowledge intentionally left outside active routing.

## Maintain

Use for every implementation task in an adopted repository.

### Route before deciding

1. Read applicable instructions.
2. Open the macro-router.
3. Map the task to every relevant concern.
4. Traverse matching routes once each.
5. Read required leaves.
6. Load other priorities only when conditions hold.
7. Inspect targeted implementation evidence.

### Resolve knowledge during work

- Apply each rule only inside its declared scope.
- Report material conflict between normative authority and descriptive evidence.
- Do not promote an observed local example to project-wide policy.
- Do not create documentation for a speculative solution.
- Preserve new reusable findings only after verification.

### Run the knowledge-impact check

After implementation verification, inspect the actual changes and answer every lifecycle question in `protocol.md`.

If no knowledge changes, record `No knowledge update required` and a concrete reason.

If knowledge changes:

1. update the existing canonical owner when possible;
2. apply `authoring-standard.md`;
3. update affected routes and ancestors;
4. update or remove affected local annotations;
5. remove or demote superseded text;
6. validate structural, routing, and semantic effects;
7. report implementation and knowledge verification separately.

Routine maintenance MUST remain scoped to the affected knowledge. It does not authorize a repository-wide audit.

## Audit

Default to read-only diagnosis unless repair is explicitly requested.

### Define scope

Choose one: whole repository, domain, changed paths, routing layer, authoring quality, freshness, or protocol conformance.

### Inspect structure

Trace from the macro-router and identify:

- missing targets;
- unreachable active artifacts;
- duplicate canonical owners;
- cycles or repeated traversal;
- stale paths;
- routers containing leaf knowledge;
- leaves registered without triggers or skip conditions.

### Inspect routing

Exercise representative tasks:

- clear positive match;
- plausible near-miss;
- multi-domain task;
- ambiguous task;
- documentation-maintenance task.

Record concerns, selected and excluded routes, selected leaves, and rationale.

### Inspect semantics and quality

1. Compare sampled important claims with approved intent and current evidence.
2. Check status and uncertainty labels.
3. Check canonical terminology and duplicated authority.
4. Apply the granularity and quality gates from `authoring-standard.md`.
5. Distinguish proven findings from inference.

### Report or repair

Classify findings by consequence and confidence. If repair is authorized, make narrow coherent changes and rerun affected checks. Do not claim whole-system correctness from a sample.

## Upgrade

Use when moving an installed repository between approved LibrAIrian Protocol versions.

1. Read installed version and repository-local deviations.
2. Read the explicit migration for the target version.
3. Map required artifact, behavior, and terminology changes before editing.
4. Preserve project knowledge, approved policy, unrelated instructions, and history.
5. Separate protocol mechanics from substantive knowledge correction when that improves reviewability.
6. Validate the repository-local fallback without depending on the skill.
7. Update the version only after required checks pass.

Stop and request a decision if no migration exists or the upgrade would reinterpret project policy.

## Required handover

Report:

- procedure and scope;
- protocol version before and after;
- knowledge added, updated, moved, merged, split, demoted, or removed;
- evidence used and conflicts found;
- structural, routing, semantic, and operational checks actually run;
- unresolved uncertainty;
- knowledge-impact result.

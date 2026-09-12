---
name: librairian
description: Apply LibrAIrian Protocol when initializing, adopting, maintaining, auditing, or upgrading repository-local AI knowledge, including routers, knowledge leaves, authoring rules, and knowledge-impact review. Do not use for ordinary human documentation that is not part of the project knowledge system.
---

# LibrAIrian

Apply LibrAIrian Protocol 1.0.0 so agents receive the smallest complete repository context for each task and maintain that knowledge with the implementation.

## Select the procedure

- **Initialize** a new or nearly empty repository.
- **Adopt** the protocol in a mature repository.
- **Maintain** knowledge affected or discovered by an implementation task.
- **Audit** routing, authority, correctness, freshness, granularity, duplication, or value.
- **Upgrade** an existing installation through an explicit migration.

Read [references/procedures.md](references/procedures.md) for the selected procedure.

Read [references/protocol.md](references/protocol.md) before Initialize, Adopt, Audit, Upgrade, or any change to protocol mechanics or terminology.

Read [references/authoring-standard.md](references/authoring-standard.md) when creating, rewriting, splitting, merging, translating, compressing, or substantially reviewing AI-facing documentation.

Read [references/templates.md](references/templates.md) only when installing or changing protocol artifacts.

## Required operating rules

1. Find the repository root and every applicable instruction file.
2. If `Docs/ai/ROUTER.md` exists, open it before classifying the task against repository concerns.
3. Follow every applicable route. Routing is cumulative, never first-match.
4. Read required knowledge before making project-specific decisions. Load recommended and reference material only when its conditions hold.
5. Resolve every explicit inherited-default dependency of the selected leaves. Apply documented local exceptions and overrides; do not infer missing ones.
6. Combine selected knowledge with targeted code, test, configuration, schema, version-control, and runtime evidence.
7. Separate normative authority from descriptive evidence. Report material conflicts instead of silently choosing one side.
8. Persist only verified or explicitly uncertain knowledge that is reusable, non-obvious, durable, canonically owned, and worth its context and maintenance cost.
9. Routers select. Leaves explain. Cross-links compose knowledge without copying authority.
10. Preserve approved project policy, unrelated instructions, user authorization boundaries, and existing work.
11. Do not add speculative capabilities, unapproved tooling, task logs, secrets, personal data, generic framework guidance, or cheaply derivable source structure to project knowledge.

## Completion contract

Every implementation task using this skill MUST perform the knowledge-impact check after implementation verification.

When knowledge changes:

- update the canonical owner in the same change;
- remove or demote stale knowledge;
- update affected owning and ancestor routers;
- apply the authoring quality gate;
- run repository-provided structural checks and focused routing, inheritance, and semantic verification;
- report implementation verification and knowledge verification separately.

`No knowledge update required` is valid only with a concrete reason.

Do not claim that link, graph, or syntax checks prove semantic truth, freshness, or route completeness.

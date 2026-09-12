# Hydra Agent Instructions

These rules bind repository work unless higher-priority platform instructions
or explicit authorized user requirements supersede them. Preserve approved
product intent; derive implementation conventions from verified evidence.

## LibrAIrian Protocol

Before making project-specific decisions, open
[Docs/ai/ROUTER.md](Docs/ai/ROUTER.md), classify every concern, and follow every
applicable route. Routing is cumulative, never first-match. Read required leaves
and their explicit inherited defaults before deciding or changing behavior.
The macro-router and child routers own documentation paths and selection triggers;
this file MUST NOT become a second domain route inventory.

Use the repository-local `librairian` skill at
`.agents/skills/librairian/SKILL.md`. When unavailable, follow the Governance
fallback selected by the same macro-router. No machine-local skill path is required.

Normative authority is the authorized task, approved requirements and decisions,
then scoped repository policy. Code, tests, configuration, runtime, and Git history
are descriptive evidence, not competing policy. A more specific rule supersedes
another only through an explicit relationship. Report material conflicts and
stop the affected decision; do not silently redefine intent from implementation.

## Execution safeguards

- Inspect the actual repository and existing work before implementation.
- Preserve safety, recoverability, Git compatibility, and Head isolation.
- Develop production behavior in Rust with mandatory Red-Green-Refactor and
  explicit regression assessment. Observe the focused failure before production
  implementation; obtain explicit user authorization before an unavoidable deviation.
- Use disposable repositories for destructive tests. Never target this checkout,
  a real user project, or an existing Head with destructive integration tests.
- Do not commit without user authorization, rewrite shared history, bypass hooks,
  or include unrelated changes. Follow routed commit conventions before any
  commit creation, amendment, squash, review, or proposal.
- Destructive or ambiguous repair requires explicit confirmation. Preserve
  uncertain refs, paths, ownership, metadata, and work rather than guessing.
- Deliver the smallest complete, verified change. Follow the Development route
  for detailed engineering, authorization, test, dependency, and completion rules.

## Knowledge maintenance is part of completion

Every implementation task MUST perform the knowledge-impact check after its
implementation verification. Update the canonical owner and affected owning and
ancestor routers in the same change. Remove or demote superseded rules, resolve
explicit inherited defaults, and run structural, routing, semantic, and operational
verification proportionate to the changed knowledge.

Every non-trivial task MUST assess the distributed Hydra skill and user-documentation
impact. User-visible changes update the affected English documentation and maintained
Italian guide together. Agent-operable changes also update the canonical Hydra skill.
Follow the routed Development and Governance contracts for the exact scope.

`No knowledge update required` is valid only with a concrete reason. Report
implementation verification separately from knowledge verification, including
remaining uncertainty. Links and syntax checks do not prove semantic correctness,
freshness, or complete routing. Do not call a task complete while an applicable
implementation, regression, documentation, or skill obligation remains unmet.

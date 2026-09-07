# Project Knowledge Governance Router

## Purpose and scope

Route tasks that install, maintain, audit, or upgrade LibrAIrian Protocol and
Hydra's AI-facing project knowledge. This router does not own product,
architecture, or development rules.

## Selection rules

| Conditions | Priority | Read | Skip when |
|---|---|---|---|
| any: protocol adoption, router change, knowledge maintenance, documentation authoring or review, audit, upgrade, fallback operation | required | [librairian-protocol.md](librairian-protocol.md) | ordinary implementation after all affected project routes are selected |

## Cross-context composition

Add every domain router whose knowledge is being created, changed, moved,
demoted, or removed.

## Ownership and maintenance

This router canonically owns `librairian-protocol.md`. Update the macro-router
only when the governance boundary or top-level path changes.

# Project Knowledge Governance Router

## Purpose and scope

Route tasks that install, maintain, audit, or upgrade LibrAIrian Protocol and
Hydra's AI-facing project knowledge. This router does not own product,
architecture, or development rules.

## Selection rules

| Conditions | Priority | Read | Skip when |
|---|---|---|---|
| any: implementation completion, knowledge-impact check, protocol adoption, router change, knowledge maintenance, AI-knowledge authoring or review, audit, upgrade, fallback operation | required | [librairian-protocol.md](librairian-protocol.md) | initial implementation investigation before knowledge maintenance or completion |
| any: knowledge change, protocol installation, audit, knowledge-preservation review, routing or inheritance change, fallback verification | required | [knowledge-validation.md](knowledge-validation.md) | a completed impact check identifies no changed knowledge with a concrete reason |

## Cross-context composition

Add every domain router whose knowledge is being created, changed, moved,
demoted, or removed.

## Ownership and maintenance

This router canonically owns `librairian-protocol.md` and
`knowledge-validation.md`. Update this router and every affected ancestor for
changes in owned artifacts, paths, or selection boundaries.

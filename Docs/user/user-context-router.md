# User Documentation Context Router

## Purpose

This router owns Hydra's end-user documentation: installation during
development, supported command workflows, configuration guidance,
customization, safety expectations, troubleshooting, and clearly labelled
future capabilities.

User documentation is written in Italian unless a separate localized document
is intentionally introduced. It explains how to use released or currently
implemented behavior and does not redefine product or architecture contracts.

---

## Routes

| Document | Consult when the task involves | Nature of the document |
|---|---|---|
| [`hydra-user-guide.it.md`](hydra-user-guide.it.md) | A user-visible command, option, default, output, workflow, configuration field, customization, safety constraint, supported platform, troubleshooting case, or planned capability described to users | Maintained Italian user guide. It documents the usable workflow and advanced configuration, while separating implemented behavior from planned functionality. |

---

## Consultation Rules

- Consult the user guide for every change that alters externally observable
  CLI or configuration behavior.
- Update the guide in the same change when a command, option, default, output,
  workflow step, validation rule, supported customization, or user recovery
  action changes.
- Describe only behavior verified in the current implementation as available.
  Planned product contracts may be mentioned only under an explicit
  **non disponibile** or **pianificato** label.
- Keep examples copyable and consistent with `hydra --help` and the
  implementation.
- Combine this route with the product router when scope or intended behavior
  changes, and with the architecture router when implementation state or known
  limitations determine what users can safely do.

Routes are cumulative. User documentation translates authoritative contracts
into operating guidance; it does not override them.

---

## Domain Maintenance

Add another user document only when it has a stable audience or responsibility
that would make the main guide materially easier to navigate.

When adding, renaming, moving, or removing a document:

1. update the **Routes** table;
2. update consultation rules when ownership changes;
3. update the parent router at
   [`../hydra-context-router.md`](../hydra-context-router.md);
4. verify every relative link and command example.

Do not store implementation notes, task logs, or speculative command syntax in
this domain.

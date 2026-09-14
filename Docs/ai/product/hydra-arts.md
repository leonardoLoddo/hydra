# Hydra Arts Product Contract

**Status:** current
**Scope:** adaptive agent strategies built on isolated, disposable Hydra Heads
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read when naming, positioning, adding, changing, invoking, or documenting a
Hydra Art. Skip for ordinary Head lifecycle work that does not use or change an
Art.

## Product boundary

Hydra provides isolated, disposable working realities inside a Git project.
Hydra Arts convert that isolation into reusable problem-solving strategies for
AI coding agents. They do not add CLI commands, hooks, orchestration services,
or another execution runtime. Agents and humans continue to use ordinary Hydra
Head lifecycle commands.

The user-facing term is **Hydra Arts**. Technical descriptions may call them
adaptive strategies, workflow patterns, or agent strategies. Do not call them
hooks or present them as fixed pipelines.

## Shared Art policy

Hydra Arts are intent-driven, not step-driven. An agent MUST use the lightest
execution that preserves the selected Art's intent and invariants. It MUST skip
actions that do not materially improve the result and MUST NOT invoke an Art
ceremonially.

An Art supports three activation modes:

- **Explicit:** the user requests the Art.
- **Suggested:** the agent explains why an Art may repay its cost and asks the
  user to choose when the value is plausible but uncertain.
- **Autonomous:** the agent invokes an Art when prerequisites are clear, it
  materially reduces uncertainty or risk, and its cost and disruption are
  proportionate.

Arts preserve Hydra's existing authorization and safety boundaries. Selecting
an Art does not by itself authorize integration, target-ref mutation, forced
removal, or discarding identified work unless the user's request clearly
includes that action.

## Current Arts

| Art | Technical description | Governing question | Minimum invariant |
|---|---|---|---|
| Arena | Competitive Implementation | What is the strongest implementation? | Compare at least two materially distinct contenders from the same relevant baseline, keep them isolated until evaluation, crown one, and salvage useful discoveries before cleanup. |
| Augury | Experimental Design | Is this idea worth building, and how should it behave? | Use a disposable Head to test one real uncertainty, optimize for learning, probe meaningful weaknesses, stop when enough is known, and preserve findings rather than experimental mess. |
| Gauntlet | Adversarial Validation | Where does this implementation break or fall short? | Start from an existing implementation, attack its highest-risk assumptions, prefer reproducible evidence, preserve useful proof artifacts, and stop at diminishing returns. |

Arts are independent and MAY compose when evidence creates a useful transition.
They are not a required linear lifecycle.

## Operational projection

`skills/hydra/SKILL.md` owns Art recognition and routing. The dedicated files
under `skills/hydra/references/` own detailed agent guidance. The repository
README and maintained user guides explain the capability to humans. Release
archives and the CLI skill installer MUST distribute the complete canonical
skill tree.

## Verification

Validate the skill structure and links. Install the managed skill into a
temporary provider home and verify every reference is present in its provenance
manifest. Build a release archive and verify it contains the same references.
Confirm README and user-guide descriptions preserve the names, activation
modes, adaptive policy, core intent, and lifecycle authorization boundaries.

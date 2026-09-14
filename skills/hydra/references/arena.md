# Arena — Competitive Implementation

> Let the heads compete. Crown the strongest.

Use Arena to decide between at least two materially different implementation
strategies by comparing working results from the same relevant baseline. Do not
use it for cosmetic variations or when one approach is already clearly
required by the project.

## Intent and triggers

Arena converts meaningful implementation uncertainty into evidence. It is
useful when credible alternatives differ in architecture, abstraction, data
model, algorithm, dependency choice, locality, or another consequential
trade-off, and implementing contenders costs less than continuing to guess.

Use only as many Heads as there are credible theses. Two may be enough. Do not
create contenders to fill a predetermined count.

Before growing Heads, state each thesis distinctly, for example:

```text
minimal local change
domain-oriented refactor
framework-native implementation
```

Variations of the same idea are one contender, not several.

## Invariants

- Start every contender from the same relevant commit.
- Give every contender the same prerequisites, constraints, and acceptance
  criteria.
- Keep implementations independent until evaluation. A contender MUST NOT see
  or influence another contender's work before judging begins.
- When the environment supports independent subagents and current policy allows
  them, one subagent per contender can reduce reasoning contamination. The same
  agent may work sequentially when subagents are unavailable.
- Keep every edit, test, and temporary artifact inside its own Head.
- Apply the repository's implementation and verification standards to each
  candidate unless the comparison explicitly investigates one of those
  standards.

## Adaptive execution

Use the normal Hydra guidance in `../SKILL.md` to inspect, grow, enter, and
validate each Head. Minimize shared reasoning between contenders. Record only
the information needed to judge them fairly.

Choose comparison criteria from the actual problem. Relevant criteria may
include correctness, completeness, regression protection, simplicity, diff
locality, architectural fit, maintainability, performance, dependency cost,
operational complexity, and domain-specific constraints. Weight them by
importance; do not force an equal-weight scorecard when direct judgment is
clearer.

The shortest, most sophisticated, fastest, or most heavily tested contender is
not automatically the winner. Crown the implementation with the strongest
overall trade-off for this task and explain the decisive evidence.

## Plunder before severing

Inspect losing Heads before cleanup. Salvage useful tests, counterexamples,
edge cases, benchmarks, implementation details, and project knowledge that
improve the winner. A losing implementation can still contain the best test.

Carry the winner forward through Hydra's normal handoff or integration flow.
Crowning does not authorize integration, target-ref mutation, forced removal,
or discarding files. Preserve those authorization boundaries from
`../SKILL.md`.

The default outcome is a crowned candidate and clean disposal of losing Heads
after useful discoveries are preserved. Inspect each Head before removal. If
the Art invocation does not clearly authorize cleanup, ask before removing it.
Never force removal without explicit authorization for the identified files.

## Exit condition

Stop when the credible contenders have been compared, a winner can be
explained from evidence, and useful discoveries have been preserved. Do not
continue merely to make the competition more elaborate.

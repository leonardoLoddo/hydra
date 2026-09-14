# Augury — Experimental Design

> Walk the future. Return with knowledge.

Use Augury during design or brainstorming when a disposable experiment can
answer an important question more cheaply and reliably than further theoretical
discussion. Its primary output is knowledge, not reusable production code.

## Intent and triggers

Create an isolated Head to experience enough of a possible feature,
architecture, interaction, dependency, or integration to make a better design
decision. Use it for real uncertainty: feasibility, architectural fit, UX,
latency, data shape, state, lifecycle, or a difficult external boundary.

Do not use Augury when reading existing evidence or running a tiny local check
already answers the question. Optimize for:

```text
information gained / work spent
```

## Invariants

- Name the uncertainty before building.
- Use a disposable isolated Head from a relevant committed baseline.
- Build only enough to answer the uncertainty.
- Keep shortcuts realistic enough that they do not invalidate the experiment.
- Probe meaningful weaknesses once the critical path exists.
- Stop as soon as the evidence is sufficient.
- Preserve findings; discard experimental mess by default.

## Adaptive experimentation

Use the normal Hydra guidance in `../SKILL.md` to create and enter the Head.
Optimize for learning rather than production completeness. When it improves the
experiment, temporary code may use hardcoded values, mock data, one
representative path, rough fixtures, invasive logging, instrumentation,
exploratory tests, or temporary schema and configuration changes.

Do not take a shortcut through the condition being tested. A dependency spike
that mocks the dependency's difficult behavior proves nothing about that
behavior. A UX experiment still needs enough real interaction to judge the UX.

Once the idea can be exercised, challenge relevant weak points such as empty,
missing, partial, invalid, or large data; permissions; latency and retries;
concurrency; state transitions; lifecycle failures; backward compatibility;
unexpected user paths; performance cliffs; and architectural friction. Select
only the cases that can change the design decision.

Iterate inside the disposable Head when one result exposes a better question.
Do not prescribe an iteration count. If multiple credible implementation
strategies emerge and comparison would add value, suggest or transition to
Arena by reading `arena.md`; do not force the composition.

## Return with knowledge

Capture the findings in the lightest useful form. Include validated and
invalidated assumptions, discovered constraints and edge cases, architectural
or UX observations, risks, and the recommended direction when supported.

Do not promote prototype code because it already exists. Reuse it only through
a deliberate decision and only after it satisfies the real implementation's
standards. Otherwise discard the Head after preserving the evidence. If the Art
invocation does not clearly authorize cleanup, ask before removal, and never
force removal without explicit authorization for the identified files.

## Exit condition

Stop as soon as the original uncertainty is resolved well enough to decide.
An experiment that invalidates one key assumption may already be complete.

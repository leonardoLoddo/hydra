# Gauntlet — Adversarial Validation

> Make the implementation earn its survival.

Use Gauntlet on an existing implementation when ordinary review and green
happy-path tests do not provide enough confidence. Actively try to expose
defects, weak assumptions, misleading tests, regressions, architectural
friction, and unnecessary machinery.

## Intent and triggers

Gauntlet is diagnosis through adversarial evidence. Do not merely check whether
something might be wrong. Try to prove what is wrong. If no concrete defect can
be demonstrated, identify the strongest remaining weaknesses and improvement
opportunities, and report which important attacks the implementation survived.

Use a disposable Hydra Head when attacks may mutate code, tests, data, timing,
configuration, or state. Keep the implementation under evaluation safe while
the attack Head is abused.

## Invariants

- Start from the exact implementation being evaluated.
- Assume passing happy-path tests are insufficient evidence.
- Rank risk surfaces before attacking them.
- Prefer reproducible evidence over speculative warnings.
- Tie hardening work directly to an exposed weakness.
- Preserve useful proof artifacts before cleanup.
- Stop when further attacks have diminishing value relative to the remaining
  risk.

## Attack library

Choose attacks that fit the domain. This is a library, not a checklist.

- **Behavior:** empty, missing, malformed, duplicate, stale, minimum, maximum,
  ordering, invalid transition, repeated action, retry, idempotency, partial
  success, interruption, concurrency, race, time, and lifecycle cases.
- **Test quality:** invert important conditions, change comparison operators,
  bypass validation or authorization, alter return values, remove side effects,
  force exceptions, or skip branches. A meaningful surviving mutation exposes
  a test gap.
- **Input space:** property-based cases, fuzzing, extreme sizes, random
  combinations, nullability, Unicode, duplicate identifiers, and unexpected
  but valid ordering.
- **Architecture:** boundary violations, hidden coupling, duplicated logic,
  incidental dependencies, unnecessary abstraction, poor locality, convention
  drift, and equivalent behavior achievable with materially less machinery.
- **Operations:** unavailable dependencies, timeouts, partial transactions,
  crashes, duplicate jobs, stale caches, permission changes, resource pressure,
  and fault injection.
- **UX:** repeated input, navigation or reload during work, slow networks,
  empty/loading/error states, conflicting actions, keyboard access,
  accessibility, and rendered behavior that tests do not exercise.

Attack the highest-risk assumptions first. Do not maximize the number of
checks. For a parser, malformed input and properties may dominate; for a
state-changing operation, idempotency, interruption, concurrency, and rollback
may matter more.

Use the normal Hydra guidance in `../SKILL.md` to inspect, create, enter, and
eventually clean up the attack Head. Destructive lifecycle tests must target
new disposable repositories, never the Hydra source repository or user work.

## Proof and hardening

Prefer findings that leave a durable artifact:

```text
reproduced bug       -> regression test
surviving mutation   -> assertion that kills it
performance cliff    -> benchmark
race or state defect -> deterministic reproduction
excess complexity    -> smaller demonstrated alternative
```

Useful result categories are `Broken`, `Weak`, `Hardened`, and `Survived`, but
use them only when they make the evidence easier to understand. Gauntlet may
produce fixes, stronger tests, or a simpler implementation. It MUST NOT turn a
finding into an unrelated rewrite.

Preserve useful tests and findings before discarding the attack Head. If the
Art invocation does not clearly authorize cleanup, ask before removal, and
never force removal without explicit authorization for the identified files.

## Exit condition

Stop when the important risks have been challenged, findings already justify a
return to implementation, or additional attacks are unlikely to repay their
cost.

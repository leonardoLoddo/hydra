# Knowledge Validation

**Status:** current
**Scope:** repeatable validation of Hydra's project-knowledge graph and activation
**Canonical owner:** [ROUTER.md](ROUTER.md)

## Consult when

Read for knowledge changes, protocol installation, audits, route or inheritance
changes, and fallback verification. Skip for a completed knowledge-impact check
that identifies no changed knowledge and gives a concrete reason.

## Structural check

Run the following from the repository root with Python 3. It needs only the standard
library and does not modify repository files. It checks concrete local Markdown
links and heading anchors, leaf metadata, owning-router registration, and reachable
AI documents. Code-fenced examples are excluded from link checks. External links
are not validated by this command.

```bash
python3 - <<'PYCODE'
from pathlib import Path
import re
from urllib.parse import unquote

root = Path.cwd().resolve()
ai = root / "Docs/ai"
entry = ai / "ROUTER.md"
active = set(ai.rglob("*.md"))
artifacts = active | set((root / ".agents/skills/librairian").rglob("*.md"))
artifacts |= set((root / "skills/hydra").rglob("*.md"))
artifacts |= {root / "AGENTS.md", root / "CONTRIBUTING.md", root / "README.md"}
artifacts |= set((root / "Docs/user").rglob("*.md"))
errors, graph = [], {}

def prose(path):
    return re.sub(r"^```[^\n]*\n.*?^```[ \t]*$", "", path.read_text(),
                  flags=re.M | re.S)

def anchors(path):
    found, counts = set(), {}
    for heading in re.findall(r"^#{1,6} +(.+)$", prose(path), re.M):
        base = re.sub(r"[^\w\- ]", "", heading.lower()).replace(" ", "-")
        index = counts.get(base, 0)
        counts[base] = index + 1
        found.add(base if index == 0 else f"{base}-{index}")
    return found

for path in sorted(artifacts):
    text = prose(path)
    graph[path] = set()
    for target in re.findall(r"\]\(([^)]+)\)", text):
        if re.match(r"[a-zA-Z][a-zA-Z0-9+.-]*:", target):
            continue
        file, _, fragment = unquote(target.strip("<>")).partition("#")
        dest = (path.parent / file).resolve() if file else path
        if not dest.exists():
            errors.append(f"{path.relative_to(root)}: missing {target}")
        elif fragment and dest.suffix == ".md" and fragment not in anchors(dest):
            errors.append(f"{path.relative_to(root)}: missing anchor {target}")
        if dest in active:
            graph[path].add(dest)
    if path in active and path.name != "ROUTER.md":
        for field in ("Status", "Scope", "Canonical owner"):
            if not re.search(r"^\*\*" + field + r":\*\* .+", text, re.M):
                errors.append(f"{path.relative_to(root)}: missing {field}")
        if not re.search(r"^\*\*Status:\*\* (current|proposed|disputed|historical)$", text, re.M):
            errors.append(f"{path.relative_to(root)}: invalid status")
        if "## Consult when" not in text:
            errors.append(f"{path.relative_to(root)}: missing consultation trigger")
        owner = path.parent / "ROUTER.md"
        if text.count("**Canonical owner:** [ROUTER.md](ROUTER.md)") != 1:
            errors.append(f"{path.relative_to(root)}: ambiguous canonical owner")
        if not owner.exists() or not re.search(
            r"^\|.*\]\(" + re.escape(path.name) + r"\).*$", prose(owner), re.M
        ):
            errors.append(f"{path.relative_to(root)}: missing owning selection row")
seen, pending = set(), [entry]
while pending:
    path = pending.pop()
    if path not in seen:
        seen.add(path)
        pending.extend(graph.get(path, set()) - seen)
for path in sorted(active - seen):
    errors.append(f"{path.relative_to(root)}: unreachable")
assert not errors, "\n".join(errors)
print(f"Validated {len(active)} AI documents and {len(artifacts)} Markdown artifacts")
PYCODE
```

Run `git diff --check` as well. Review routing tables for positive conditions,
priority, and near-miss exclusions. Confirm every folder containing active leaves
has its own router. Cross-domain navigation can return to a visited router; process
it once. An inheritance cycle or competing canonical owner is an error, not a
reason to recurse indefinitely.

This checker recognizes the repository's current inline-link and heading style.
When introducing reference-style links, HTML anchors, or another router format,
extend the check or verify those constructs explicitly; do not treat unparsed
markup as validated.

## Routing and inheritance regression cases

Exercise these cases by reading the actual router conditions. Selection is semantic
judgment, not keyword matching. Record selected and excluded leaves with rationale.
All selected inherited defaults must enter context once, with explicit local
exceptions applied. Add or revise a case when a change creates a difficult boundary.

| Case | Expected required context | Excluded context and reason |
|---|---|---|
| Change native storage-probe cleanup without changing init or create | Product storage plus inherited core safety; Architecture doctor; Development working agreements and Rust; Governance impact check at completion | Roadmap, close, and recipes do not govern diagnostic cleanup |
| Review Rust code without editing it | Development working agreements and Rust; Product and Architecture only for the reviewed contracts | Read-only review still makes engineering judgments; absence of a diff does not skip Rust/TDD rules |
| Propose a different persistence technology for Head state | Architecture system boundary; Product state and inherited core safety; Development working agreements and dependency rules | A technology proposal does not authorize migration or change existing persisted formats |
| Reformat a Rust source without behavior changes | Development working agreements and Rust; Governance impact check | Product behavior and Architecture workflow leaves have no changed contract |
| Change persistent unsafe-overlay exclusions | Product configuration, storage, lifecycle, state, CLI, inherited core safety; Architecture creation and materialization; Development working agreements, Rust, Hydra skill; Governance maintenance | Roadmap is not authorization to add overlay profiles or recipes |
| Change protected removal used by close | Product lifecycle and state plus inherited safety; Architecture removal and close; Development working agreements, Rust, Hydra skill; Governance maintenance | Open adapter and future assisted merging remain unrelated |
| Run close from a Head | Product lifecycle plus core safety; Architecture close for implementation reasoning | The close caller exception extends parent normalization and rejects the call; normalization does not authorize close from a Head |
| Correct an Italian user-guide typo without changing a command or rule | Development working agreements if non-trivial; user documentation route; record no technical knowledge impact | Product roadmap and AI authoring rules do not govern ordinary human prose |
| Split a product knowledge leaf | Governance fallback and validation; affected Product leaves and defaults; Development working agreements | Unaffected Architecture workflows need no rewrite; ownership and ancestor routes must change |
| Request portable recipes | Product roadmap and current core boundary; Development working agreements if proposing repository changes | Proposed syntax is not current CLI behavior or permission to implement without an explicit decision |
| Request unspecified “sync Heads” behavior | Product core and lifecycle for investigation; roadmap only if future synchronization is intended | Do not infer automatic rebase, copy, integration, or ref changes; clarify a material behavior ambiguity |

For inherited rules, additionally verify that a missing source or conflicting
applicability is surfaced, not silently ignored. Check force removal only replaces
the clean-worktree and prior-integration preconditions for worktree removal while
preserving unintegrated branches; private-branch deletion still requires integration.
Verify that unsafe-symlink exclusion changes selected policy
without weakening the rejection of unsafe copied entries.

## Semantic and historical review

For each rewritten or moved unit, compare its old content from Git history and
record where every durable requirement, exception, rationale, future design
constraint, and known gap went. Use a source-to-destination table for a substantial
rewrite, including each numbered acceptance criterion and each edge-case list.
Classify each unit as retained, moved, deliberately excluded with a reason, or
missing. Do not equate availability in Git history with preservation in active
knowledge. Keep the task-specific audit outside the active knowledge graph.
Removing duplication is valid only when a reachable canonical owner retains the
rule. Keep historical task records and superseded examples outside active routes.

Inspect relevant code, tests, configuration, and explicit decisions for changed
claims. Pay particular attention to parent context versus close caller restriction,
overlay origin, required configuration fields, recovery evidence, force semantics,
platform limits, and proposed versus available capabilities. An old policy differs
from current evidence only by an explicit decision, not by silently preferring code.

Run targeted disposable-repository tests when rewritten operational claims need
verification. Documentation-only edits do not require invented Rust tests or a
production TDD cycle. Report semantic sampling honestly; passing links cannot
prove truth or completeness.

## Operational and skill verification

- Follow `AGENTS.md` into the macro-router, affected domains, and Governance at
  completion. Confirm no task needs a developer's machine-local skill path.
- Simulate a host unable to load `.agents/skills/librairian/`: follow the macro-router
  to the Governance fallback, then this leaf. Verify routing, inheritance,
  authoring, authority, impact review, and validation are usable without the skill.
- Validate both `.agents/skills/librairian/` and `skills/hydra/` with an available
  Agent Skill validator. Parse front matter and `agents/openai.yaml` with a YAML
  parser; inspect names, descriptions, and all bundled resource paths. Report an
  unavailable validator or parser and the exact alternative checks used.
- For an explicitly authorized LibrAIrian installation or refresh, compare every
  runtime file with the approved distribution. Exclude OS litter such as `.DS_Store`.
  Synchronize fallback mechanics and protocol markers. Do not invent a version bump.
- Inspect the actual diff for user-documentation and Hydra operational-skill impact.
  A governance-only change does not require a cosmetic operational skill edit.

## Completion evidence

Report structural results, representative routing and inheritance outcomes,
semantic evidence and remaining uncertainty, and operational activation separately.
State a concrete reason when user documentation or the Hydra skill needs no change.
The protocol guides agents through repository instructions; it does not install a
background maintainer, generator, hook, or CI service by itself.

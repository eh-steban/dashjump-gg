# [Bug Summary] Fix

> **File location:** `private/plans/fixes/`
> **File naming:** `kebab-case` describing the bug -- e.g., `midboss-damage-dispatch-panic.md`

## When to use this template

Use a bugfix plan when **all** of these hold:

- The problem is a specific, reproducible defect (not a feature, not a refactor)
- The root cause is already identified or strongly suspected
- The fix is narrow: one focused change, possibly across services, but no new capabilities

If the fix requires exploring unknowns, use a spike first. If it's a typo or a one-line config change, use `/quick-fix` and skip this template entirely. If the work adds new behavior or spans multiple unrelated changes, use `implementation.md`.

---

## Context

[1-3 sentences: what is broken from a user/system perspective, how it was discovered. Link the spike, issue, log, or report that surfaced it.]

**Discovered via:** [spike | production log | user report | test failure | code review | ...]
**Related:** [path to spike/issue/prior plan, or "none"]
**Overall confidence:** `confirmed` / `inferred` / `hypothesis` *(per `.claude/rules/research.md`)* -- holistic read on the whole plan: do you believe the fix as scoped will resolve the symptom without surprise? Distinct from the Root Cause confidence below, which only rates the diagnosis. If `inferred` or `hypothesis`, name what would upgrade it (a probe run, a spike, a verified reproduction) and whether the fix is safe to ship without that evidence.

---

## Symptom

[What a reader can observe to confirm the bug exists. Exact log line, exception message, failing test name, UI glitch with steps to reproduce. Specific enough that "I can't reproduce it" becomes a fixable problem, not a stalemate.]

**Reproduction:**
1. [Step]
2. [Step]
3. [Observed vs expected]

---

## Root Cause

[The actual mechanism. Cite `file:line`. Explain *why* the current code produces the symptom, not just *what* is wrong.]

**Confidence:** `confirmed` / `inferred` / `hypothesis` *(per `.claude/rules/research.md`)*

If confidence is below `confirmed`, note what additional evidence would upgrade it and whether the fix is safe to ship without that evidence.

---

## Scope

| Service  | Involved | Agent            |
|----------|----------|------------------|
| Parser   | yes / no | `rust-parser`    |
| Backend  | yes / no | `backend-python` |
| Frontend | yes / no | `frontend-react` |

**Contract change:** yes / no

If `yes`, update the relevant spec in `private/specs/contracts/` *before* touching implementation code. List the specs below:

- [ ] `private/specs/contracts/[spec].md` -- [fields changing]

### Contract pre-check *(only if Contract change: yes)*

Before editing the contract spec or any code, verify the **source values** the spec already carries are correct. Stale or partially-updated reference tables in a contract spec are a classic way to re-introduce the same bug under a different mask -- downstream code derives from the spec, so a wrong row in the spec guarantees a wrong line in the implementation.

For each field changing, list:
- Where the source-of-truth value lives (probe output, upstream proto, vendored reference doc)
- Every row/value in the spec that depends on it
- A checkbox confirming each row was verified byte-for-byte against the source

```
- [ ] Pre-check completed: [N/N values verified in `path/to/spec.md:<lines>`]
```

If any value is missing or mismatched, **stop** and update the spec to the verified source values before continuing.

---

## Fix

[Specific changes, file by file. Use action verbs: add, remove, rename, change type. Include enough detail that an agent can execute without follow-up questions.]

**Files to change:**
- `[path/to/file]` -- [what changes]
- `[...]` -- [...]

**Out of scope** *(things a reader might expect to see fixed here but shouldn't be):*
- [Item] -- *Reason: [why it's being deferred]*

---

## Acceptance Criteria

Group ACs by how they're verified. Don't conflate "the test suite proves it" with "a human must look at it" -- a checkbox that requires both is two checkboxes that block each other.

*Verifiable by test suite (automated, deterministic, runnable in CI):*
- [ ] Regression test added that would have caught this bug *(see Testing below)*
- [ ] [Specific assertion #1]
- [ ] [Specific assertion #2 -- one per behavior the fix promises]

*Verifiable by manual check (humans must observe; goes in the Verification table below):*
- [ ] Symptom from Symptom section no longer reproduces when re-running the reproduction steps
- [ ] [Any UI / log / network observation that can't be automated]

*Process:*
- [ ] Contract spec updated *(if Contract change: yes)*
- [ ] Project Definition of Done met: tests, observability, conventions, security

---

## Testing

### Regression test *(required)*

Every bugfix plan must add a test that fails against the unpatched code and passes after the fix. Describe it here before writing it:

- **Test file:** `[path]`
- **What it asserts:** [specific behavior]
- **Why it would have caught the bug:** [what gap in existing coverage this closes]

### Existing tests affected
- [Test file] -- [needs update because ...]

---

## Verification

*(Remove rows for services not in scope. All commands run inside containers per `.claude/CLAUDE.md` -- Runtime section.)*

| Service  | Command                                          | Expected                      |
|----------|--------------------------------------------------|-------------------------------|
| Parser   | `docker compose exec dashjump-parser cargo test` | [specific pass criteria]      |
| Backend  | `docker compose exec dashjump-backend pytest`    | [specific pass criteria]      |
| Frontend | `docker compose exec dashjump-frontend npm test` | [specific pass criteria]      |

**Manual check:** [specific action and expected outcome -- e.g., "parse replay 55423930 via `GET /match/analysis/55423930` and confirm no panic in parser logs"]

---

## Learnings *(optional)*

If the bug reveals a pattern future agents should know (a class of bug, a process gap, a missing convention), append a draft to `private/learnings.md` ## Drafts. Skip this section if the fix is isolated and doesn't generalize.

Cite the **evidence** that proves the pattern is real, not speculative. A learning without a citation is a hunch -- future agents won't be able to tell whether the rule still applies or whether the world has moved on. Good citations include: an in-codebase precedent (`file:line` of the right way to do it), a spike whose probe runs validated the mechanism, a production log or incident report, or a contract spec that documents the convention. Bad citations: "I think this is true", "everyone knows", or no citation at all.

- [ ] Draft appended to `private/learnings.md` ## Drafts
- [ ] Pattern identified: [one line, or "none -- isolated fix"]
- [ ] Evidence cited: [in-codebase precedent `file:line` | spike path | log/incident reference | contract spec citation]

---

## Execution Order

1. Fill in Context, Symptom, Root Cause, Scope, and Fix **before** touching code
2. If `Contract change: yes`, update the contract spec and pause for user review
3. Implement the fix per Scope; write the regression test alongside
4. Run per-service verification commands from Verification table
5. Run `test-auditor` and `code-reviewer` agents against the unstaged diff *(skip only for true one-liners)*
6. Append learnings draft if applicable
7. User review -- commit

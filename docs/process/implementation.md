# [Feature Name] Plan

> **File location:** `private/plans/implementation/`
> **File naming:** use descriptive `kebab-case` matching the feature -- e.g., `lane-creep-tracking-refactor.md`

## Context

[1-3 sentence problem statement. What is broken or missing, and why does it matter?]

**Goals:**
- [Measurable outcome 1]
- [Measurable outcome 2]

**Branch:** `feature/[branch-name]` *(branching from main to isolate from active feature work? see `.claude/rules/git.md` -- Worktree Workflow)*
**Review workflow:** implement -- test -- subagent updates plan -- pause for user review -- commit -- next phase

---

## Scope

Declare which services are involved. Agents must read this before starting -- skip any phase
whose service is marked **no**.

| Service  | Involved | Agent              |
|----------|----------|--------------------|
| Parser   | yes / no | `rust-parser`      |
| Backend  | yes / no | `backend-python`   |
| Frontend | yes / no | `frontend-react`   |

Remove rows for uninvolved services from Critical Files, phases, and Verification Summary below.

---

## Acceptance Criteria

Feature is done when ALL of the following are true from a user/product perspective:

- [ ] [User-visible behavior -- e.g., "Individual creep dots appear on minimap at correct positions"]
- [ ] [Data contract -- e.g., "API response includes `lane_creep_data` key with per-creep timelines"]
- [ ] [Bug fix verification -- e.g., "No double-wave appears during zipline descent"]
- [ ] All in-scope phase checkpoints complete and signed off by user

These are separate from the project-wide Definition of Done (code review, tests, observability,
security, conventions) which applies to every unit of work.

---

## Reference Data *(optional -- delete if not needed)*

[Domain lookup tables, constants, formulas relevant to this feature.]

---

## Critical Files

*(Remove rows for services not in scope)*

| Layer | File | Change |
|-------|------|--------|
| Parser constants | `parser/src/entities/constants.rs` | Modify |
| Parser domain | `parser/src/domain/[file].rs` | Create / Modify |
| Parser tracker | `parser/src/tracking/[file].rs` | Create / Modify |
| Parser integration | `parser/src/replay_parser.rs` | Modify |
| Backend domain | `backend/app/domain/[file].py` | Create / Modify |
| Backend service | `backend/app/services/[file].py` | Create / Modify |
| Backend use case | `backend/app/application/use_cases/[file].py` | Modify |
| Frontend domain | `frontend/src/domain/[file].ts` | Create / Modify |
| Frontend component | `frontend/src/components/[path].tsx` | Create / Modify |
| Frontend page | `frontend/src/pages/[file].tsx` | Modify |

---

## Phase 0 -- Contract (`backend-python` agent, or whoever owns the boundary)

> **Skip this phase if only one service is in scope.**
>
> This phase blocks all others. No parallel work begins until the contract checkpoint is signed off.

### 0.1. Define or update affected contract specs

For each service boundary this feature crosses, update the relevant spec in `private/specs/contracts/`:

- Parser output changes: update `parser-output.md` (owned by `rust-parser`)
- Backend API changes: update `backend-api.md` (owned by `backend-python`)

Spec updates must include: field name, type, required/optional, and a notes column entry.

### 0.2. Verify consuming-service alignment

For each updated spec, confirm the consuming service's current types still compile against it.
List any fields that will change and which service needs updating in Phase B or C.

### 0 Checkpoint

**Status:** `[ ] Not started` / `[ ] In progress` / `[ ] Complete` / `[ ] Blocked`

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. List every field added, removed, or renamed across service boundaries
> 2. Confirm the relevant contract spec(s) have been updated
> 3. Check off every item below with date and actual result
> 4. Await user review before any Phase A/B/C work begins

#### Results *(agent fills in)*

- [ ] Contract spec(s) updated -- [list files changed]
- [ ] All boundary-crossing field changes listed below
- [ ] Consuming service types confirmed compatible (or noted as requiring update in next phase)

#### Field change log *(agent fills in)*

| Field | Change | Spec file | Consuming service impact |
|-------|--------|-----------|--------------------------|
| [field_name] | [added / removed / renamed from X to Y] | [spec file] | [what needs updating] |

#### Deferred items
[None, or list with reason]

Await user review before proceeding to Phase A.

---

## Phase A -- Parser (`rust-parser` agent)

> **Skip this entire phase if Parser is not in scope.**

### A1. [Step name]

[What to do and why. Specific enough that the agent can act without follow-up questions.]

### A2. [Step name]

[...]

### A[n]. Record learnings

Append important findings to `private/learnings.md` ## Drafts. Examples: entity lifecycle
discoveries, why one approach was chosen over another, game-engine behavior future agents should know.

Also surface any magic numbers introduced: list each constant, its value, and its source or derivation
in the checkpoint summary below. If the source is unknown, add a `# TODO: verify` comment inline.

### A Checkpoint

**Status:** `[ ] Not started` / `[ ] In progress` / `[ ] Complete` / `[ ] Blocked`

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. Run `cargo test` and record results below
> 2. Parse a real replay and record sample output below
> 3. Check off every item below -- add date and actual result inline, not just a tick
> 4. Note any deferred items with reason
> 5. Update **Status** above to reflect current state

#### Results *(agent fills in)*

- [ ] `cargo test` -- [X passed, Y failed -- paste failure output if any]
- [ ] [Specific behavior verified -- e.g., "4 creeps per wave confirmed in JSON output"]
- [ ] [Another specific check]
- [ ] Learnings appended to `private/learnings.md`

#### Sample output *(agent fills in)*
```
[Paste relevant snippet -- e.g., first 60s of a creep timeline, or key test output]
```

#### Deferred items
[None, or list with reason]

Await user review and commit approval before proceeding to Phase B.

---

## Phase B -- Backend (`backend-python` agent)

> **Skip this entire phase if Backend is not in scope.**

### B1. [Step name]

[...]

### B[n]. Record learnings

Append findings to `private/learnings.md` ## Drafts. Surface any magic numbers introduced: list each
constant, its value, and its source or derivation in the checkpoint summary. If unknown, add a
`# TODO: verify` comment inline.

### B Checkpoint

**Status:** `[ ] Not started` / `[ ] In progress` / `[ ] Complete` / `[ ] Blocked`

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. Run `pytest` and record results below
> 2. Hit the relevant API endpoint and record a sample response below
> 3. Check off every item below with date and actual result
> 4. Note any deferred items with reason
> 5. Update **Status** above

#### Results *(agent fills in)*

- [ ] `pytest` -- [X passed, Y failed]
- [ ] [Specific API behavior verified -- e.g., "`lane_pressure` key present in response"]
- [ ] Learnings appended to `private/learnings.md`

#### Sample output *(agent fills in)*
```
[Paste relevant API response snippet]
```

#### Deferred items
[None, or list with reason]

Await user review and commit approval before proceeding to Phase C.

---

## Phase C -- Frontend (`frontend-react` agent)

> **Skip this entire phase if Frontend is not in scope.**

### C1. [Step name]

[...]

### C[n]. Record learnings

Append findings to `private/learnings.md` ## Drafts. Surface any magic numbers introduced: list each
constant, its value, and its source or derivation in the checkpoint summary. If unknown, add a
`# TODO: verify` comment inline.

### C Checkpoint

**Status:** `[ ] Not started` / `[ ] In progress` / `[ ] Complete` / `[ ] Blocked`

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. Run `npm test` and record results below
> 2. List manual UI verification steps completed
> 3. Check off every item below with date and actual result
> 4. Note any deferred items with reason
> 5. Update **Status** above

#### Results *(agent fills in)*

- [ ] `npm test` -- [X passed, Y failed]
- [ ] [Specific UI behavior verified -- e.g., "creep dots visible and move with timeline scrub"]
- [ ] Learnings appended to `private/learnings.md`

#### Deferred items
[None, or list with reason]

Await user review and commit approval.

---

## Verification Summary

*(Remove rows for services not in scope)*

| Phase | Command | Key checks | Status |
|-------|---------|------------|--------|
| 0 | Contract spec review | All boundary fields documented, consuming service types confirmed | |
| A | `cargo test` | [What to verify] | |
| A | Parse real replay | [What to verify] | |
| B | `pytest` | [What to verify] | |
| B | API spot-check | [What to verify] | |
| C | `npm test` | [What to verify] | |
| C | Manual UI | [What to verify] | |

---

## Execution Order

*(Omit steps for services not in scope)*

0. **Plan review** -- run `spec-writer` agent to review this plan for template alignment, completeness, measurable acceptance criteria, learnings citations, and contract field coverage -- fix before proceeding
1. **Phase 0** (contract) -- user review -- proceed *(blocks all phases; skip if single-service)*
2. **Phase A** (rust-parser) -- test-auditor + code-reviewer -- user review -- commit
3. **Phase B** (backend-python) -- test-auditor + code-reviewer -- user review -- commit *(depends on A's output schema)*
4. **Phase C** (frontend-react) -- test-auditor + code-reviewer -- user review -- commit *(depends on B's API shape)*

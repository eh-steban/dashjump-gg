# [Question Title] Spike

> **File location:** `private/plans/spikes/`
> **File naming:** `kebab-case` describing the question -- e.g., `damage-field-value-range.md`

## Context

[1-2 sentences: what question are we answering and why it matters now]

---

## Scope check

Before filling this out, confirm the work actually fits a spike. If any of the following are true, stop and use `discovery.md` instead:

- The answer will be a table, a decision tree, or a recommendation across more than one field / message / query.
- You can already list more than one open question without repeating yourself.
- The output will inform a data-model or architectural decision rather than close a single factual gap.
- The investigation will span more than one agent's expertise.

A spike answers **one** factual question with **one** citation, in ≤ 1 day, and does not gate implementation design. Everything else is a discovery.

---

## Question

[Single precise question -- specific enough that data + a direct answer counts as done]

---

## Assumptions

### To Validate

List assumptions the investigation will actively test. Each assumption must specify the **evidence type** required to mark it confirmed.

**Cap: 3 assumptions.** More than that means you are answering more than one question -- promote to `discovery.md` before the investigation begins, not after.

Evidence types (in order of strength):
- `code-run` -- ran a probe/test against a real replay and observed output (required for any claim about message availability, field values, or entity behavior)
- `type-check` -- verified struct/field exists in generated types (necessary but not sufficient -- confirms the field is defined, not that it appears in replays)
- `doc-read` -- read documentation or reference material (lowest confidence -- use only for context, never to confirm runtime behavior)

**A finding is only `confirmed` if it has `code-run` evidence.** Findings backed only by `type-check` or `doc-read` are `inferred` at best.

- [ ] [Assumption] -- *Evidence required: [`code-run` | `type-check`]* -- *How to check: [specific probe, test, or command to run]*

### Accepted (not tested here)

List assumptions taken as given for this spike. Agents proceed on these -- but must not treat them
as confirmed facts.

- [Assumption] -- *Risk if wrong: [what would break or change]*

---

## Agent & Timebox

**Agent:** [haste-expert | backend-python | rust-parser | frontend-react | ...]
**Timebox:** [e.g., 2 hours]

---

## Research Standards

Follow `.claude/rules/research.md` for confidence labels, citation format, and scope discipline.

---

## Investigation Approach

[What data source / file / replay / API / query to inspect. Specific enough to act on without
follow-up questions.]

---

## Findings *(agent fills in)*

**Answer:** [Direct answer to the question above]

**Supporting evidence:**
- [citation: file:line or proto field or URL] -- [evidence type: `code-run` | `type-check` | `doc-read`] -- [what was observed]
- [...]

**Overall confidence:** `confirmed` / `inferred` / `hypothesis`

A finding is `confirmed` only when backed by `code-run` evidence. `type-check` alone = `inferred`. `doc-read` alone = `hypothesis`.

### Assumptions check

- [ ] [Assumption from "To Validate"] -- held / invalidated -- [evidence type] -- [what was observed]
- Accepted assumptions worth flagging based on findings: [none, or note]

### Probe / test artifacts

List the probe binary, test, or command that was run, and where to find the output or how to reproduce it.

- [Probe/test name] -- [how to run] -- [where output is saved or what to look for]

---

## Graduating this spike *(agent fills in)*

Pick **one** next step and write a one-sentence justification. The choice should follow directly from the Findings + confidence label above -- if you cannot justify it from what you wrote in Findings, the spike is not done.

- [ ] **Fix plan** (`private/plans/fixes/`) -- one specific defect, root cause confirmed, remedy is narrow and obvious. *Example: spike found a single off-by-one in `lane_pressure_service.py` and the fix is "change `range(n)` to `range(n+1)`".*
- [ ] **Discovery plan** (`private/plans/discovery/`) -- headline question answered, but two or more unknowns still block implementation design. *Example: spike confirmed parser exposes mid-boss spawn events, but the storage shape, transform-layer placement, and frontend visualization are all undecided.*
- [ ] **Implementation plan** (`private/plans/implementation/`) -- all assumptions validated, design is obvious, scope is "build the thing we just designed". *Example: spike validated the proto field, the schema, and the existing transform path -- next step is wiring it through.*
- [ ] **Another spike** -- answer is `inferred` or `hypothesis` and the next question is itself narrow. *Example: spike narrowed the suspect to one of two parser code paths, and the next spike is "which one fires first?".*
- [ ] **Stop** -- investigation-only with no follow-up. *Example: spike answered "what does this proto field mean?" for general knowledge; no code change implied.*

**Rationale:** [one sentence tying the choice to a specific finding above]

---

## Learnings Output

- [ ] Draft entry appended to `private/learnings.md` ## Drafts
- [ ] Follow-up questions or spikes needed:

  [None, or list]

---

## Plan Review

Run `spec-writer` agent after filling in Findings to review: template alignment, confidence labels applied correctly, assumptions checked against findings, learnings drafted, and follow-up spikes identified where confidence is below `confirmed`.

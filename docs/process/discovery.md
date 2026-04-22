# [Topic] Discovery

> **File location:** `private/plans/discovery/`
> **File naming:** `kebab-case` describing the topic -- e.g., `damage-event-data-model.md`

## Context

[What must be known before implementation can begin, and why this can't wait]

**Goals:** What decisions will this discovery inform?

- [Decision 1 -- e.g., "data model for damage events"]
- [Decision 2 -- e.g., "whether to derive X from replay or pull from API"]

---

## Open Questions

Be specific -- vague questions produce vague answers.

1. [Question -- specific enough that a data citation closes it]
2. [Question]
3. [Question]

---

## Assumptions

### To Validate

List assumptions the investigation will actively test. If one is invalidated, findings must say
so explicitly -- a wrong assumption can change the entire conclusion.

- [ ] [Assumption] -- *How to check: [specific data source or query]*

### Accepted (not tested here)

List assumptions taken as given. Testing them is out of scope, too expensive, or unknowable here.
Agents proceed on these -- but must not treat them as confirmed facts.

- [Assumption] -- *Risk if wrong: [what would break or change]*

---

## Agent Assignments

| Question(s) | Agent | Approach |
|-------------|-------|----------|
| 1, 2 | haste-expert | Inspect proto fields in valveprotos-rs |
| 3 | backend-python | Query against real match data |

---

## Research Standards

Follow `.claude/rules/research.md` for confidence labels, citation format, and scope discipline.

---

## Investigation Approach

[What to inspect, what data to use, what scripts or queries to run. Include specific replay files, API endpoints, or DB tables to target.]

### Phases (optional, for sequential investigations)

Use phases when later questions depend on earlier answers, or when you need to isolate variables before combining them. Skip this section for flat investigations.

- **Phase 1 -- [name]:** what this phase isolates; probe binary or query; expected output
- **Phase 2 -- [name]:** what it adds; depends on Phase 1 result X
- **Phase 3 -- [name]:** ...

---

## Decision tree (fill in during planning, not after)

Map plausible finding combinations to the path forward. Writing this before running probes forces you to know what each outcome would mean -- and catches investigations where every branch leads to the same answer.

| Finding A | Finding B | Path forward |
|-----------|-----------|--------------|
| Yes | Yes | [approach 1] |
| Yes | No | [approach 2] |
| No | Yes | [approach 3] |
| No | No | [further investigation: what and why] |

---

## Probe / Query Artifacts (agent fills in)

List the probe binary, test, script, or query that was run, where its output lives, and how to reproduce.

- [artifact name] -- [how to run] -- [where output is saved or what to look for]

---

## Discovery Checkpoint *(agent fills in)*

**Status:** `[ ] Not started`

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. Answer every open question with confidence labels and citations
> 2. Record assumption validation results
> 3. Paste representative evidence below
> 4. Append findings to `private/learnings.md` ## Drafts
> 5. Update **Status** above

### Results

- [ ] Q1: [answer] -- [confidence] -- [citation]
- [ ] Q2: [answer] -- [confidence] -- [citation]
- [ ] Q3: [answer] -- [confidence] -- [citation]
- [ ] Learnings appended to `private/learnings.md`

### Assumptions check

- [ ] [Assumption from "To Validate"] -- held / invalidated -- [evidence]
- Accepted assumptions worth flagging based on findings: [none, or note]

### Evidence

```
[Paste relevant field values, query output, proto excerpts, or sample data]
```

### Deferred questions

[None, or list with reason and what would resolve them]

---

**STOP. Present the following to the user before doing anything else:**

1. Answers to every open question with confidence labels
2. Data model or approach recommendation based on findings
3. Enrichment opportunities worth flagging for later
4. Unresolved questions and what would resolve them

Await user decision. If approved, create an implementation plan before writing any code.

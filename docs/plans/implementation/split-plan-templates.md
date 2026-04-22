# Split Plan Templates

## Context

The current `private/templates/plan.md` bundles research and implementation into one document via an
optional Phase 0. When agents skip Phase 0, they jump directly to Phase A/B with no label transition,
and the phase naming breaks down. More fundamentally, research work and implementation work have
different shapes -- different agents, different outputs, different review ceremonies -- and conflating
them in one template produces awkward plans.

The fix is to introduce three purpose-built templates in a dedicated `private/templates/plans/`
subdirectory, mirror that structure in `private/plans/`, and update CLAUDE.md so agents always pick
the right template.

---

## What Changes

### 1. File & directory layout

**Before:**
```
private/
  templates/
    plan.md
  plans/
    *.md (flat)
```

**After:**
```
private/
  templates/
    plans/
      spike.md          (new)
      discovery.md      (new)
      implementation.md (renamed from plan.md, Phase 0 removed)
  plans/
    spikes/             (new dir)
    discovery/          (new dir)
    implementation/     (new dir, existing plans moved here)
```

Existing flat plans in `private/plans/` stay where they are -- no migration of historical files.
New plans are created in the appropriate subdirectory going forward.

Also rename this plan file from `jaunty-wobbling-nebula.md` → `split-plan-templates.md` during
implementation.

---

### 2. `private/templates/plans/spike.md` (new)

For narrow, timeboxed questions -- "what values does this field take?", "does the API expose X?".
One question, one agent, one day or less. Produces a draft learnings entry.

Key sections:
- Context (1-2 sentences)
- Question (single, precise)
- Assumptions (split: testable vs. accepted -- see §5 below)
- Agent + timebox
- Investigation approach
- Findings *(agent fills in)* -- confidence labels + citations per `research.md`
- Learnings output

No checkpoint ceremony -- agent fills in findings and is done.

---

### 3. `private/templates/plans/discovery.md` (new)

For multi-question investigations that must resolve unknowns *before* implementation can be designed.
Replaces Phase 0 of the current template.

Key sections:
- Context + goals (what decisions this discovery will inform)
- Open questions (specific, numbered)
- Assumptions (split: testable vs. accepted -- see §5 below)
- Agent assignments
- Research standards reminder (ref `research.md`)
- Investigation approach
- Discovery Checkpoint *(agent fills in + stops for user review)*
- Recommendation to user

Mandatory stop at checkpoint: agent presents findings with confidence labels and awaits a go/no-go
before any implementation plan is created.

---

### 4. `private/templates/plans/implementation.md` (modified from plan.md)

The existing implementation plan. Phase 0 block removed entirely -- unknowns are assumed already
resolved by a prior spike or discovery doc. Phases start at A with no skip or awkward gap.

---

### 5. Assumptions structure (in spike.md and discovery.md)

Assumptions are split into two categories:

**To Validate** -- beliefs the investigation can directly test. Agents must check these during
the investigation and record whether they held. If one is invalidated, findings must say so
explicitly -- an invalidated assumption can change the entire conclusion.

**Accepted** -- beliefs held as given for this investigation. Testing them is either out of scope,
too expensive, or truly unknowable here. Agents must not silently rely on these; listing them with
their risk-if-wrong keeps findings honest and flags where they could break.

Template section structure:

```markdown
## Assumptions

### To Validate
List assumptions the investigation will actively test. Mark each one in the Findings/Checkpoint.

- [ ] [Assumption] -- *How to check: [specific test or data source]*

### Accepted (not tested here)
List assumptions taken as given. Agents proceed on these -- but must not treat them as confirmed.

- [Assumption] -- *Risk if wrong: [what would break or change]*
```

In the checkpoint, agents must include:
- Which "To Validate" assumptions held or were invalidated (with evidence)
- Any "Accepted" assumptions that seem questionable based on findings (flag only -- don't expand scope)

---

### 6. CLAUDE.md update

Replace the current single-line plan reference with:
- A 3-row table mapping situation → template
- Updated paths reflecting the `private/templates/plans/` subdirectory
- Updated paths reflecting the `private/plans/{spikes,discovery,implementation}/` structure

---

## Critical Files

| File | Change |
|------|--------|
| `private/templates/plans/implementation.md` | Create (copy plan.md, remove Phase 0 block) |
| `private/templates/plans/spike.md` | Create new |
| `private/templates/plans/discovery.md` | Create new |
| `private/templates/plan.md` | Delete (superseded) |
| `private/plans/spikes/` | Create directory (add `.gitkeep`) |
| `private/plans/discovery/` | Create directory (add `.gitkeep`) |
| `private/plans/implementation/` | Create directory for migrated plans |
| `private/plans/implementation/archived/` | Move from `private/plans/archived/` |
| `private/plans/implementation/completed/` | Move from `private/plans/completed/` |
| All flat `*.md` in `private/plans/` | Move into `private/plans/implementation/` |
| `private/plans/jaunty-wobbling-nebula.md` | Move + rename → `private/plans/implementation/split-plan-templates.md` |
| `.claude/CLAUDE.md` | Update Workflow > Plans section |

---

## Template Content Outlines

### spike.md

```
# [Question Title] Spike

> File location: private/plans/spikes/

## Context
[1-2 sentences: what question and why it matters now]

## Question
[Single precise question -- specific enough that data + a direct answer counts as done]

## Assumptions

### To Validate
- [ ] [Assumption] -- *How to check: ...*

### Accepted (not tested here)
- [Assumption] -- *Risk if wrong: ...*

## Agent & Timebox
Agent: [haste-expert | backend-python | ...]
Timebox: [e.g., 2 hours]

## Investigation Approach
[What data source / file / replay / API to inspect]

## Findings *(agent fills in)*

Follow research.md confidence labels (confirmed / inferred / hypothesis) and citation format.

**Answer:** [Direct answer]

**Supporting evidence:**
- [citation] -- [what was observed]

**Confidence:** confirmed / inferred / hypothesis

**Assumptions check:**
- [ ] [Assumption 1] -- held / invalidated -- [evidence]

## Learnings Output
- [ ] Draft entry appended to `private/learnings.md` ## Drafts
- [ ] Follow-up questions noted:
  [None, or list]
```

### discovery.md

```
# [Topic] Discovery

> File location: private/plans/discovery/

## Context
[What must be known before implementation can begin, and why this can't wait]

**Goals:** What decisions will this discovery inform?
- [Decision 1]
- [Decision 2]

## Open Questions
1. [Question -- specific enough that a data citation closes it]
2. [Question]

## Assumptions

### To Validate
- [ ] [Assumption] -- *How to check: ...*

### Accepted (not tested here)
- [Assumption] -- *Risk if wrong: ...*

## Agent Assignments
| Question(s) | Agent | Approach |
|-------------|-------|----------|
| 1 | haste-expert | Inspect proto in valveprotos-rs |
| 2 | backend-python | Query against real match data |

## Research Standards
Follow `.claude/rules/research.md`:
- Label every claim: confirmed / inferred / hypothesis
- Cite every factual claim (file:line, proto field, URL)
- Say "I don't know" when uncertain -- flag gap and how to fill it

## Investigation Approach
[What to inspect, what data to use, what scripts/tools to run]

---

## Discovery Checkpoint *(agent fills in)*

**Status:** `[ ] Not started`

> Agent instructions: Stop here. Before returning you MUST:
> 1. Answer every open question with confidence labels and citations
> 2. Record assumption validation results
> 3. Paste representative evidence below
> 4. Append findings to `private/learnings.md` ## Drafts
> 5. Update Status above

### Results
- [ ] Q1: [answer] -- [confidence] -- [citation]
- [ ] Q2: [answer] -- [confidence] -- [citation]
- [ ] Learnings appended to `private/learnings.md`

### Assumptions check
- [ ] [Assumption] -- held / invalidated -- [evidence]
- Accepted assumptions worth flagging: [none, or note]

### Evidence
[Paste field values, query output, or proto excerpts]

### Deferred questions
[None, or list with reason and what would resolve them]

**STOP. Present to user before doing anything else:**
1. Answers to every open question with confidence labels
2. Data model / approach recommendation
3. Enrichment opportunities worth flagging
4. Unresolved questions and what would resolve them

Await user decision. If approved, create an implementation plan before writing any code.
```

### CLAUDE.md change (Workflow > Plans line)

Replace:
```
- Plans: `private/plans/` -- use `private/templates/plan.md` when creating a new plan
```

With:
```
- Plans: pick the right template from `private/templates/plans/`:

  | Template | File location | When to use |
  |----------|---------------|-------------|
  | `spike.md` | `private/plans/spikes/` | Single narrow question, timebox ≤ 1 day, no implementation |
  | `discovery.md` | `private/plans/discovery/` | Multiple unknowns blocking implementation design |
  | `implementation.md` | `private/plans/implementation/` | Implementation where unknowns are already resolved |
```

---

## Verification

No runtime tests -- docs/templates only. Verify by:

1. `private/templates/plans/` contains exactly `spike.md`, `discovery.md`, `implementation.md`
2. `private/templates/plan.md` is deleted
3. `private/plans/` contains `spikes/`, `discovery/`, `implementation/` directories (and nothing else)
4. `private/plans/implementation/` contains all previously flat `.md` files plus `archived/` and `completed/` subdirs
5. `implementation.md` template has no Phase 0 section
6. Both `spike.md` and `discovery.md` reference `research.md` and contain the two-category Assumptions section
7. CLAUDE.md Workflow section contains the 3-row table with correct subdirectory paths

# Knowledge Management System

This document defines how to capture, organize, and reference knowledge across the dashJump.gg project. It prevents valuable discoveries from being lost and enables Claude instances to discover relevant knowledge when needed.

**Key principle:** Learnings load on-demand based on what you're working on, not all at once. Use `private/learnings-index.md` to find what's relevant.

---

## Quick Start: Where Does This Discovery Go?

### Decision Matrix

| Discovery Type | Destination | When to Use | Size |
|---|---|---|---|
| Cross-project pattern (appears 2+ times) | `private/learnings.md` | "We keep making this mistake" across services | 20-40 lines |
| Service-specific architecture constraint | `.claude/rules/[service]/[service]-mental-model.md` | Full explanation unique to one service | 200-400 lines |
| Feature-specific requirement/assumption | `private/specs/NNN-feature.md` → Assumptions section | This feature depends on X being true | 5-15 lines |
| Experiment outcome/validation | `private/product/experiments/NNN/learnings.md` | After experiment reaches terminal status | 20-50 lines |
| Code-level implementation detail | Inline code comment | Points to where detailed info lives | 1-3 lines |

### Quick Decision Flow

```
You discover something important
    ↓
Have you seen this pattern 2+ times?
    → Yes → Does it affect multiple services?
            ↳ Yes → private/learnings.md (link to mental model)
            ↳ No  → .claude/rules/[service]/[service]-mental-model.md
    → No  → Is it service-specific architecture?
            ↳ Yes → .claude/rules/[service]/[service]-mental-model.md
            ↳ No  → Is it feature-specific?
                    ↳ Yes → private/specs/NNN-feature.md → Assumptions
                    ↳ No  → Inline code comment
```

---

## When to Check `private/learnings.md`

Don't load all learnings at once. Instead, check the index BEFORE:
- Starting work on any timeline-related feature
- Making storage/data decisions
- Building features coaches requested in interviews
- Debugging issues that seem architectural or cross-service
- Implementing anything in parser that touches frame/time logic

**Process:**
1. Read `private/learnings-index.md` (find relevant topic)
2. If a learning applies, load that entry from `private/learnings.md`
3. Follow links to mental models for deeper context

---

## The Five Tiers Explained

### Tier 1: `private/learnings.md` (Cross-Project Discoveries)

**What goes here:** Discoveries that affect multiple services or prevent repeated mistakes

**Characteristics:**
- Pattern has appeared 2+ times (or would save 2+ hours if next person knows about it)
- Affects 2+ services or critical to understanding system
- High-impact insight (prevents debugging cycles)
- Cross-project (not specific to one feature or experiment)

**Lifespan:** Permanent reference material
**Maintenance:** Update when new pattern identified (check quarterly for stale entries)
**Discovery mechanism:** Use `private/learnings-index.md` to find relevant learnings

**Ownership and write protocol:**
- **Promoted learnings (above ## Drafts):** `spec-writer` agent only
- **## Drafts section:** Any service agent may append raw findings
- **private/learnings-index.md:** `spec-writer` agent only, updated during /consolidate-learnings
- Service agents discovering a pattern append to ## Drafts with format:
  `### [Draft] [Topic] — [agent: agent-name, date: YYYY-MM-DD]`
- `spec-writer` reviews, promotes valid drafts, discards duplicates/incorrect findings

**When to add:**
- "We keep forgetting this" (pattern appears 2nd time)
- "This affects multiple services" (architectural constraint crossing boundaries)
- "This would save someone hours" (expensive learning cycle preventable with upfront context)
- "Coach feedback shifts our strategy" (validated assumption with business impact)

**When NOT to add:**
- One-off bug fixes (belongs in commit message)
- Single-feature assumptions (belongs in spec)
- Service-only gotchas (belongs in mental model)
- Implementation details (belongs in code)

**Format:** See "Learnings Entry Format" section below

---

### Tier 2: `.claude/rules/[service]/[service]-mental-model.md` (Service-Specific Architecture)

**What goes here:** Deep dives into service-specific architecture, gotchas, and patterns

**Characteristics:**
- Non-obvious architectural constraint in one service
- Expensive debugging cycle if not documented
- Patterns emerging from service-specific code review
- Data flow or encoding quirk unique to this service

**Lifespan:** Permanent reference material (updated when architecture changes)
**Maintenance:** Created during debugging or architectural review; updated as service evolves

**When to create:**
- You've debugged the same issue in one service 2+ times
- Architectural decision seems counterintuitive (needs explanation)
- Data encoding or structure is non-standard
- Interaction between subsystems within one service is complex

**Content structure:**
1. Core Concept — What makes this service unique
2. Architecture Constraints — Decisions that seem odd but are necessary
3. Data Flow/Assumptions — How data moves through the service
4. Common Gotchas — Mistakes developers make and why they're wrong
5. Debugging Checklist — How to verify things are working correctly

**Linked from:**
- `private/learnings.md` entries that reference service-specific details
- Specs that depend on this service's behavior
- Code comments that point to architectural explanations

**Examples:**
- `.claude/rules/parser/parser-mental-model.md` — Demo file encoding, timeline offset, frame reconciliation
- `.claude/rules/backend/backend-mental-model.md` — Data transformation patterns, S3 storage strategy
- `.claude/rules/frontend/frontend-mental-model.md` — Visualization philosophy, timeline scrubbing logic

---

### Tier 3: `private/specs/NNN-feature.md` → Assumptions + Related Docs (Feature-Specific)

**What goes here:** Dependencies and constraints specific to this feature

**When to add:**
- Specifying any new feature (Assumptions section is mandatory)
- Documenting data requirements (Related Docs links everything)
- Explicitly noting dependencies between services

**Key principle:** Every spec assumption should cite where that knowledge lives (learnings.md, mental model, interview notes, etc.)

---

### Tier 4: `private/product/experiments/NNN/learnings.md` (Experiment Outcomes)

**What goes here:** What you learned from completing an experiment

**When to create:**
- After `/kata-check` confirms experiment reached terminal status
- Only for Product Kata experiments (not every debugging session)

---

### Tier 5: Code Comments (Implementation Details)

**What goes here:** Pointers to foundational knowledge, implementation gotchas

**Format:**
```python
# CRITICAL: Demo files record from pre-game. See .claude/rules/parser/parser-mental-model.md
# Timeline reconciliation must use game_start_marker + game_clock, not array index
game_time = (position_index - game_start_marker) / FRAMES_PER_SECOND
```

---

## Learnings Entry Format Template

```markdown
## [Title: Concise statement of the discovery]

**Date discovered:** [Month Year, Context where discovered]
**Impact:** [Which services/teams affected — be specific]
**Status:** [active | deprecated | investigating | pattern-identified | validated]

[One paragraph explaining the core insight]

**Key Takeaway:**
[One sentence: The actionable insight or rule of thumb]

**Related Docs:**
- [Link to detailed mental model if exists]
- [Link to specs that depend on this]
- [Link to experiment or interview notes that validated this]

**When to Reference:**
[Bullet list of situations where this learning applies]

**Prevention:**
[Checkmarks for how to prevent forgetting this]
```

---

## Anti-Patterns: What NOT to Do

### ✗ Duplicate Between Tiers
- Don't repeat mental model content in learnings.md (link instead)
- Don't put code comments that could be learnings.md entries

**Correct approach:**
```
learnings.md: "Demo files offset timeline. See parser-mental-model.md for full details."
parser-mental-model.md: [All the detailed content]
```

### ✗ Add Every Little Thing
- One-off bugs → code comments, not learnings
- Single-feature implementation → belongs in spec/code, not learnings
- "FastAPI requires Pydantic models" → general knowledge, not learnings

### ✗ Forget to Link
- Learnings with no source (no link to mental model or spec)
- Specs not citing applicable learnings in Assumptions
- Code comments pointing to non-existent docs

### ✗ Let Learnings.md Grow Unmaintained
- Run `/consolidate-learnings` weekly if ## Drafts has pending entries
- Review quarterly for stale/deprecated entries

---

## Maintenance Schedule

| Tier | When to Update | Owner | Action |
|---|---|---|---|
| `private/learnings.md` ## Drafts | When pattern discovered | Any service agent | Append draft finding |
| `private/learnings.md` (promoted) | Weekly via /consolidate-learnings | spec-writer only | Promote, deduplicate, prune |
| `private/learnings-index.md` | During /consolidate-learnings | spec-writer only | Add/update index entries |
| `.claude/rules/**/*.md` | During /consolidate-learnings when learnings graduate | spec-writer only | Capture permanent patterns |
| Service mental models | After debugging reveals pattern | Claude + code review | Create or enhance |
| Spec assumptions | When creating spec | spec-writer + Steven | Document dependencies |
| Experiment learnings | When experiment reaches terminal status | Claude after `/kata-check` | Document outcome |
| Code comments | Every PR touching that code | Code reviewer subagent | Keep current |

---

## Related Documents

- Archived Workflow Plan (historical context only): `private/plans/implementation/archived/dashjump-hybrid-workflow-plan.md`
- Learnings Index: `private/learnings-index.md`
- Learnings: `private/learnings.md`
- Parser Mental Model: `.claude/rules/parser/parser-mental-model.md`
- Backend Mental Model: `.claude/rules/backend/backend-mental-model.md`
- Frontend Mental Model: `.claude/rules/frontend/frontend-mental-model.md`
- Root CLAUDE.md: `.claude/CLAUDE.md`
- Project Health Skill: `.claude/skills/dashjump-context-audit/SKILL.md`
  - File ownership map: `.claude/skills/dashjump-context-audit/references/ownership-map.md`
  - Context budgets (single source of truth): `.claude/skills/dashjump-context-audit/references/context-budgets.md`

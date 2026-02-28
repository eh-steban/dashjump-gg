# Quality Criteria: Knowledge Management Files

Knowledge files manage the project's institutional memory. Three files, three roles:

- **knowledge-management.md** — The system rules (how knowledge is captured/organized)
- **private/learnings.md** — The knowledge itself (cross-project discoveries)
- **private/learnings-index.md** — The router (find relevant learnings without loading all)

## Evaluation Criteria

### knowledge-management.md

#### 1. Accuracy Against Practice
- Do the tier descriptions match how knowledge is actually being captured?
- Does the maintenance schedule reflect real cadence?
- Is the ownership table current? (spec-writer vs service agents vs Steven)

#### 2. Format Specifications
- Learning entry format is documented and unambiguous
- Drafts format is documented with agent name and date template
- Anti-patterns section is present and reflects real mistakes encountered

#### 3. Completeness
- All five tiers documented with clear "what goes here" criteria
- Promotion/graduation path described (draft → learning → rule)
- Deprecation process documented (when to archive stale learnings)

### private/learnings.md

#### 1. Entry Quality
Each promoted learning should have:
- Clear anchor ID (used by learnings-index.md and spec references)
- Concise description of the pattern/insight
- Which services are affected
- Source (how was this discovered — debugging, interview, experiment?)
- Date added or last updated
- **Anti-pattern:** Vague entries like "S3 is tricky" — must be specific

#### 2. Drafts Section Health
- Check ## Drafts for pending entries that should have been promoted
- Each draft should follow the format: `### [Draft] [Topic] — [agent: X, date: Y]`
- Stale drafts (older than 2 weeks) should be flagged for review
- Drafts that duplicate promoted learnings should be flagged for removal

#### 3. Budget
- Total learnings.md should stay under 5,000 tokens
- If approaching budget, flag oldest/least-referenced entries for potential archival
- Completed experiment learnings should reference the experiment, not repeat its content

#### 4. Graduated Entries
- If a learning has been captured as a permanent rule in .claude/rules/, the learning
  entry should note: "Graduated to: .claude/rules/{service}/{file}.md"
- Don't delete graduated learnings — they provide historical context — but mark them

### private/learnings-index.md

#### 1. Completeness
- Every promoted learning in learnings.md has a corresponding index entry
- No index entries pointing to learnings that no longer exist (orphaned pointers)

#### 2. Organization
- Entries grouped by service/topic for quick scanning
- Anchor references use the format: `learnings.md#anchor-name`
- Brief description per entry (enough to decide if you need to load the full learning)

#### 3. Budget
- Index should be lightweight — ≤500 tokens target
- If index grows large, it defeats its purpose as a quick router

## Cross-File Checks

1. **Index ↔ Learnings sync:** Every promoted learning has an index entry and vice versa
2. **Learnings → Rules graduation:** Patterns that have become permanent should be in rules/
3. **knowledge-management.md → actual practice:** Do agents actually follow the write
   protocols described? (Check: are drafts formatted correctly? Are ownership rules respected?)
4. **Spec references → Learnings:** Specs that cite learnings should use valid anchors

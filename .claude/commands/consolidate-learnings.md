Review and consolidate draft learnings using the spec-writer agent.

Steps:
1. Read private/learnings.md — check the ## Drafts section for pending entries
2. For each draft:
   - Is this a genuine cross-project pattern (2+ occurrences)?
   - Does it duplicate an existing promoted learning?
   - Is the finding accurate and well-described?
3. Promote valid drafts: move from ## Drafts to the appropriate section above,
   following the standard learning entry format in .claude/knowledge-management.md
4. Update private/learnings-index.md with new entries (add to relevant service/topic)
5. Discard duplicates or findings that turned out to be incorrect
6. Check token budget: learnings.md should stay under 5,000 tokens total

After consolidation, report:
- Learnings promoted (with anchors)
- Drafts discarded (with reason)
- Current learnings.md token estimate

Recommended cadence: weekly (pairs with /kata-check), or when ## Drafts has 3+ entries.

---
name: dashjump-context-audit
description: This skill should be used when the user asks to "audit project files", "check context health", "review .claude configuration", "scan for stale docs", or mentions "context audit" or "project memory optimization". Scans all .claude/ files and private/ documentation files, evaluates quality against type-specific rubrics, detects cross-file issues, and produces a categorized report with ownership-aware recommendations. This skill can write to spec-writer-owned files after approval.
---

# dashJump Context Audit

Audit, evaluate, and improve all project configuration and documentation files — not just
CLAUDE.md files, but agents, rules, commands, skills, and knowledge management files.

## Scan Scope

Discover all auditable files using these patterns:

```
# CLAUDE.md index files
find . -name "CLAUDE.md" -not -path "*/node_modules/*"

# Agent definitions
find .claude/agents -name "*.md"

# Slash commands
find .claude/commands -name "*.md"

# Skills
find .claude/skills -name "SKILL.md"

# Rules and mental models
find .claude/rules -name "*.md"

# Knowledge management
.claude/knowledge-management.md

# Private documentation (if accessible)
find private -name "*.md" -not -path "*/experiments/*" 2>/dev/null
```

Classify each discovered file into one of these types:
- `claude-md` — CLAUDE.md index/pointer files
- `agent` — .claude/agents/*.md definitions
- `command` — .claude/commands/*.md slash commands
- `skill` — .claude/skills/**/SKILL.md packages
- `rule` — .claude/rules/**/*.md standards and mental models
- `knowledge` — knowledge-management.md, learnings.md, learnings-index.md

## Audit Workflow

### Phase 1: Individual File Audit

For each file, load the appropriate rubric from references/:
- `claude-md` → See [references/quality-criteria-claude-md.md](references/quality-criteria-claude-md.md)
- `agent` → See [references/quality-criteria-agents.md](references/quality-criteria-agents.md)
- `command` → See [references/quality-criteria-commands.md](references/quality-criteria-commands.md)
- `skill` → See [references/quality-criteria-skills.md](references/quality-criteria-skills.md)
- `rule` → See [references/quality-criteria-rules.md](references/quality-criteria-rules.md)
- `knowledge` → See [references/quality-criteria-knowledge.md](references/quality-criteria-knowledge.md)

Score each file against the rubric. Scores are per-criterion, not an overall number.

### Phase 2: Cross-File Analysis

After individual audits, perform these cross-file checks:

**Duplication detection:**
- Identify instructions, rules, or knowledge that appears verbatim (or near-verbatim) in
  multiple files. Flag which file should be the canonical source and which should use pointers.
- Common pattern to catch: root CLAUDE.md duplicating content from rules/ files instead of
  pointing to them.

**Orphaned pointers:**
- Every file path reference (e.g., `.claude/rules/backend/testing.md`) must resolve to a
  real file. Flag dead links.
- Every anchor reference in learnings-index.md (e.g., `learnings.md#demo-timeline-offset`)
  must resolve to an actual heading in the target file.

**Ownership violations:**
- Check the ownership map: See [references/ownership-map.md](references/ownership-map.md)
- Flag if a service agent's recent git history shows writes to files it shouldn't own.

**Budget overruns:**
- Check each file against its budget: See [references/context-budgets.md](references/context-budgets.md)
- Estimate token count (lines × ~10 tokens/line as rough heuristic).

**Consistency checks:**
- Agent definitions: do all service agents have matching section structures?
  (Before Starting Work, Testing, Observability, Shared File Rules)
- Rules files: do all services have the same set of rule types?
  (mental model, testing, observability — flag missing ones)
- Commands: do file path references use private/ paths consistently?

### Phase 3: Report

ALWAYS output the report BEFORE making any changes.

```
## Project Health Report

### Summary
- Files scanned: X
- Files passing: X
- Files needing updates: X
- Cross-file issues found: X

### SPEC-WRITER ACTIONABLE (can apply after approval)
Changes to files owned by spec-writer: rules, learnings, knowledge-management.

For each finding:
- [File path]: [Issue] → [Proposed change]

### STEVEN REVIEW NEEDED (flagged, not auto-applied)
Changes to files owned by Steven: agents, commands, skills, CLAUDE.md.

For each finding:
- [File path]: [Issue] → [Recommendation]

### CROSS-FILE ISSUES
- [Duplication/Orphan/Violation]: [Details]

### FILE-BY-FILE DETAIL
Per-file scores and criterion-level notes (collapsible in long reports).
```

### Phase 4: Apply

After Steven approves:
- Apply changes to spec-writer-owned files using Edit tool
- For Steven-owned files, output the recommended diffs but do NOT apply
- Preserve existing content structure — targeted edits only, not rewrites

# Ownership Map

Defines who can write to which files. The project health skill uses this to categorize
findings into "spec-writer actionable" vs "Steven review needed" in its report.

## Spec-Writer Owned (can auto-apply after approval)

These are documentation artifacts that evolve from learnings, code review findings, and
knowledge consolidation work.

| File/Pattern | Write Protocol |
|---|---|
| `.claude/rules/**/*.md` | Update when patterns evolve or learnings graduate to rules |
| `.claude/knowledge-management.md` | Update when knowledge system process changes |
| `private/learnings.md` (above ## Drafts) | Promote valid drafts, deduplicate, prune |
| `private/learnings-index.md` | Update during /consolidate-learnings to stay in sync |
| `private/product/strategy/vision.md` | Strategic direction updates |
| `private/product/strategy/current-options.md` | Active bets and outcomes |

## Service Agent Append-Only

Service agents can only append to one specific location.

| File | Section | Format |
|---|---|---|
| `private/learnings.md` | `## Drafts` only | `### [Draft] [Topic] — [agent: name, date: YYYY-MM-DD]` |

Service agents must NOT write to: strategy files, learnings-index.md, rules files,
or any file outside their service directory (except the Drafts section above).

## Steven Owned (flagged in report, not auto-applied)

These are architectural/workflow decisions that require human judgment.

| File/Pattern | Reason |
|---|---|
| `CLAUDE.md` (root) | Defines project-wide conventions and pointers |
| `{service}/CLAUDE.md` | Defines service-specific conventions |
| `.claude/agents/*.md` | Agent responsibilities, tools, and boundaries are architectural |
| `.claude/commands/*.md` | Workflow triggers are architectural decisions |
| `.claude/skills/**/SKILL.md` | Domain knowledge packages driven by external inputs |
| `.claude/settings.json` | Configuration, rarely changes |
| `private/CONTEXT.md` | Written by /switch-machine, read by Steven at session start |

## Shared / No Owner

| File | Access Pattern |
|---|---|
| `private/plans/` | Written by Claude's plan feature, no manual ownership |
| `private/experiments/*/kata.md` | Written collaboratively during experiment work |
| `private/specs/*.md` | Drafted by spec-writer, refined collaboratively |

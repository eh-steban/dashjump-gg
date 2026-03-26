# Context Budgets

Single source of truth for file size limits. Referenced by the project health skill,
root CLAUDE.md, and the claude-md-management plugin.

## File Budgets

| File Type | Budget | Notes |
|---|---|---|
| Root CLAUDE.md | ≤200 lines (~2,000 tokens) | Index/pointers only, loads every session |
| Subdirectory CLAUDE.md | ≤150 lines each | Lazy-loads when Claude reads files in subtree |
| Agent definitions | No hard limit, target ≤150 lines | Longer agents may need scope reduction |
| Slash commands | No hard limit, target ≤80 lines | Must be self-contained |
| Skills (SKILL.md) | ≤5,000 tokens | Description loads always; body on invocation |
| Rules/mental models | Target ≤200 lines | Split by concern if exceeding |
| Spec task shards | ≤2,000 tokens per shard | Only what that unit of work needs |
| Experiment kata.md | Keep focused | Move completed steps to learnings.md |
| private/learnings.md | ≤5,000 tokens total | Archive oldest entries if approaching |
| private/learnings-index.md | ≤500 tokens | Quick router, not detailed content |
| private/CONTEXT.md | ≤2,000 tokens | Snapshot, not log — overwritten each time |

## Operational Rules

- **Clear at 30%:** Don't wait for context window to fill. Quality degrades noticeably
  past 30% context utilization. Prefer early summarization/compaction.
- **MCP servers:** Maximum 3 active simultaneously. Each adds tool definitions to the
  system prompt.
- **Subdirectory CLAUDE.md:** Lazy-loaded only when Claude reads files in that subtree.
  Don't put universal rules here -- they belong in root CLAUDE.md.
- **Skills descriptions:** Load at session start (always in context). Keep descriptions
  under ~100 words. Full skill content loads only on invocation.

## Estimation Heuristic

Approximate token count: lines × 10 tokens/line. For more precise measurement,
use `wc -w` and multiply by 1.3 (words to tokens rough ratio).

# Quality Criteria: CLAUDE.md Files

CLAUDE.md files are **index/pointer files** — they tell Claude how to find information,
not embed it. Root CLAUDE.md loads on every session. Subdirectory CLAUDE.md files
lazy-load when Claude reads files in that subtree.

## Evaluation Criteria

### 1. Pointer Integrity (Critical)
- Every file path reference resolves to an actual file
- No embedded content that belongs in rules/, skills/, or specs/
- Points to the right private/ paths (post-submodule migration)
- **Anti-pattern:** Duplicating rules inline instead of pointing to .claude/rules/

### 2. Budget Compliance
- Root CLAUDE.md: ≤200 lines (~2,000 tokens)
- Subdirectory CLAUDE.md: ≤100 lines each
- If over budget, identify what should be extracted to rules/ or skills/

### 3. Essential Sections (Root)
Check that root CLAUDE.md covers these (adapt to project needs):
- **Workflow:** How development flows (Product Kata reference, key locations)
- **Knowledge Management:** Where to find and how to update learnings
- **Shared File Ownership:** Who writes what
- **Context Budgets:** Line/token limits per file type
- **Definition of Done:** Standards for completed work
- **Installed Plugins:** List of active plugins

### 4. Essential Sections (Subdirectory)
Check that service CLAUDE.md files cover:
- **Commands:** Build, test, dev, lint — copy-paste ready
- **Architecture:** Directory structure, key files
- **Conventions:** Coding patterns specific to this service
- **Pointer to rules/:** Link to the .claude/rules/{service}/ directory

### 5. Conciseness
- Human-readable: dense is better than verbose
- No generic advice (e.g., "write clean code") — only project-specific patterns
- No aspirational content — only what's actually true about the codebase now
- Actionable commands should be copy-paste ready

### 6. Currency
- Build/test/dev commands still work
- Architecture description matches actual directory structure
- Referenced tools and dependencies are still in use
- No references to removed features or deprecated patterns

## Common Issues

1. **Content creep:** Root CLAUDE.md grows as new instructions get added without pruning
2. **Stale commands:** Build scripts change but CLAUDE.md isn't updated
3. **Inlined rules:** DDD patterns, error handling, etc. belong in .claude/rules/
4. **Missing pointers:** New rules files created without adding a reference from CLAUDE.md
5. **Outdated architecture:** File structure diagram doesn't match reality

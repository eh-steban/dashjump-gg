# Quality Criteria: Skills

Skill files (.claude/skills/**/SKILL.md) are **domain knowledge packages** — reference
material that Claude loads on invocation. The description in YAML frontmatter loads at
session start (always in context); the full content loads only when invoked.

## Evaluation Criteria

### 1. Description Quality (Critical)
The `description` field determines when Claude invokes the skill. It must:
- Use third-person: "This skill should be used when..." (not "Use this skill when...")
- List specific trigger phrases the user might say
- Be specific enough to avoid false invocations on unrelated tasks
- Be broad enough to catch all relevant invocations
- Stay under ~100 words (this loads on every session)

### 2. Budget Compliance
- Total SKILL.md content: ≤5,000 tokens
- If over budget, move detailed content to a references/ subdirectory
- Description loads always; body loads on invocation — keep the always-loaded part lean

### 3. Content Relevance
- Contains reference material, not procedural instructions (those belong in commands)
- Information is specific to this project, not generic knowledge Claude already has
- Every section earns its tokens — no filler content
- Check: would Claude perform meaningfully worse on the task WITHOUT this skill?

### 4. Currency
- Brand guidelines: do they match the current design implementation?
- Compliance requirements: are Steam ToS references current?
- Technical standards: do they reflect the actual codebase patterns?
- Check: has the domain knowledge changed since last update?

### 5. Frontmatter
Required fields:
- `name` — matches the directory name
- `description` — trigger description (see criterion 1)

### 6. Progressive Disclosure
For larger skills:
- SKILL.md stays lean with overview and key principles
- Detailed content lives in references/ subdirectory
- SKILL.md references those files with relative links

## Common Issues

1. **Trigger mismatch:** Description is too vague, causing skill to load on unrelated tasks
2. **Stale content:** Brand colors or compliance rules changed but skill wasn't updated
3. **Generic filler:** Skill contains information Claude already knows (e.g., "CSS uses selectors")
4. **Over-budget:** Skill exceeds 5,000 tokens, loading unnecessary content into context
5. **Missing skill:** A domain area (e.g., brand) has a placeholder SKILL.md that was never populated

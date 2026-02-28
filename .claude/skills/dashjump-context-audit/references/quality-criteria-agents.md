# Quality Criteria: Agent Definitions

Agent files (.claude/agents/*.md) are **behavioral contracts** — they define what an
agent does, what tools it has, what boundaries it respects, and what context it loads.

## Evaluation Criteria

### 1. Frontmatter Completeness (Critical)
Required fields in YAML frontmatter:
- `name` — matches filename (without .md)
- `description` — clear, specific trigger description for when to invoke this agent
- `tools` — explicit list (Read, Write, Edit, Bash, Glob, Grep)
- `model` — specified model tier (sonnet, opus, haiku)
Optional but recommended:
- `skills` — skills this agent should auto-load

### 2. Role Clarity
- Single clear role statement in first paragraph
- No overlap with other agents' responsibilities
- Check pairwise: does this agent's scope conflict with any other agent?
  - Service agents (backend, frontend, parser) should have non-overlapping domains
  - Utility agents (code-reviewer, spec-writer, test-auditor) should have distinct triggers

### 3. Domain Boundaries
- Agent only operates on files within its service scope
- No instructions that could lead to writing files in another agent's domain
- Service agents: confined to their service directory
- Utility agents: defined scope (e.g., test-auditor is read-only)

### 4. Required Sections (Service Agents)
- **Before Starting Work:** What context to load (learnings-index check for decision-making agents)
- **Testing (integrated):** Test responsibilities, patterns, coverage targets
- **Observability (integrated):** Logging/instrumentation conventions
- **Shared File Rules:** Write restrictions (do not write to strategy files, append to Drafts only)

### 5. Required Sections (Utility Agents)
- **Responsibilities** or **Review Checklist:** Clear scope of work
- **Shared File Ownership** (if applicable): What this agent exclusively writes
- **Output Format:** What the agent produces (for auditors/reviewers)

### 6. Tool Appropriateness
- Does the agent have tools it doesn't need? (Over-permissioned)
- Is the agent missing tools it needs? (Under-permissioned)
- Key check: spec-writer should NOT have Bash (writer, not executor)
- Key check: test-auditor should NOT have Write (read-only auditor)

### 7. Plugin References
- If relevant plugins are installed, are they listed in a "Plugins Available" section?
- Are only relevant plugins listed? (Don't list all 13 for every agent)

### 8. Consistency Across Agents
Cross-file check — all service agents should have parallel structure:
- Same section ordering
- Same "Shared File Rules" format with agent name in the draft format string
- Same "Before Starting Work" pattern (or explicitly absent for non-decision agents)

## Common Issues

1. **Scope creep:** Agent description is too broad, causing it to be invoked for wrong tasks
2. **Missing write restrictions:** Service agent can write to strategy files by omission
3. **Stale tool lists:** Agent was given Bash temporarily and it was never removed
4. **Orphaned plugin references:** Plugin was uninstalled but agent still references it
5. **Inconsistent structure:** backend-python has Observability section but frontend-react doesn't

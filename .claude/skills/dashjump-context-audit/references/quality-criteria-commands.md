# Quality Criteria: Slash Commands

Command files (.claude/commands/*.md) are **workflow triggers** — self-contained
instructions that a fresh Claude session can execute without prior context.

## Evaluation Criteria

### 1. Self-Containment (Critical)
- A fresh Claude session with no conversation history must be able to execute this command
- All file paths must be absolute or relative to project root (no assumed context)
- If the command requires a specific agent, it must name the agent explicitly
- No references to "the current experiment" or "what we discussed" — commands can't
  assume conversational state

### 2. File Path Accuracy
- All referenced paths use the correct public/private split:
  - Strategy/specs/learnings → private/
  - Rules/agents/commands/skills → .claude/
- Verify every path resolves to an actual file
- Post-migration check: no lingering docs/product/ or docs/specs/ paths

### 3. Clear Steps
- Numbered steps showing exact sequence of actions
- Each step should be unambiguous — no "review as appropriate"
- Expected outputs defined (what does success look like?)

### 4. Agent Routing
- If the command should use a specific agent, it says so explicitly
- If the command can be run by the lead agent, it shouldn't unnecessarily invoke a subagent
- Check: does the referenced agent actually have the tools needed for this command's steps?

### 5. Naming Convention
- Follows `/{action}{outcome}` pattern (e.g., /consolidate-learnings, /switch-machine)
- Name is discoverable — a user should guess the command name from what they want to do

### 6. Cadence Documentation
- If the command has a recommended frequency (weekly, monthly, end-of-session),
  it should say so
- Cadence should match what's documented in the Weekly Cadence section of root CLAUDE.md

### 7. Idempotency
- Running the command twice shouldn't produce duplicate entries or conflicting state
- Particularly important for /consolidate-learnings (promoting already-promoted drafts)
  and /switch-machine (overwriting vs appending)

## Common Issues

1. **Stale paths:** Command references pre-migration file locations
2. **Missing agent specification:** Command says "use the appropriate agent" instead of naming one
3. **Assumed context:** Command references "the active experiment" without saying how to find it
4. **Unsafe re-runs:** Running the command twice duplicates work
5. **Cadence mismatch:** Command says "weekly" but Weekly Cadence section says "monthly"

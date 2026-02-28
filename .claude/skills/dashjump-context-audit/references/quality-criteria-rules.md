# Quality Criteria: Rules and Mental Models

Rules files (.claude/rules/**/*.md) are **living standards documents** — they capture
service-specific architecture, coding patterns, testing strategies, and observability
conventions. Mental models document non-obvious system behaviors that cause bugs if
misunderstood.

## File Subtypes

### Mental Models (*-mental-model.md)
Document the "why behind the what" — non-obvious behaviors, gotchas, data flow quirks.
These prevent expensive debugging cycles by making invisible constraints visible.

### Testing Rules (testing.md)
Document service-specific test patterns, layer conventions, and what must be tested.

### Observability Rules (observability.md)
Document logging conventions, structured log formats, and instrumentation expectations.

### Cross-Service Rules (error-handling.md, observability.md at rules/ root)
Document patterns that apply across all services.

## Evaluation Criteria

### 1. Groundedness (Critical)
- Every pattern described must reflect the actual codebase, not aspirations
- Include concrete examples from real code (file paths, function signatures)
- Mental models must describe behavior that has been verified, not theorized
- **Anti-pattern:** "We should adopt X pattern" — if it's not implemented, it doesn't
  belong here

### 2. Actionability
- A developer (or agent) reading this file should know exactly what to do differently
- Rules should be specific enough to automate a check against
- Good: "Domain services must not import from infrastructure layer"
- Bad: "Keep code clean and well-organized"

### 3. Non-Obviousness
- Only document things Claude wouldn't figure out from reading the code alone
- Mental models exist because the system behavior is surprising or counterintuitive
- Good: "demo_time ≠ game_clock — positions[0] is pre-game frame, not match start"
- Bad: "The backend uses FastAPI" (Claude can see this from imports)

### 4. Concrete Examples
- At least one concrete example per rule or pattern
- Examples should reference actual project files or realistic scenarios
- Mental models should include "What goes wrong if you ignore this" scenarios

### 5. Service Isolation
- Rules in .claude/rules/backend/ should only cover backend patterns
- Rules in .claude/rules/frontend/ should only cover frontend patterns
- Cross-service patterns belong in .claude/rules/ root, not duplicated per service

### 6. Completeness Across Services
Cross-file check — each service should have:
- mental model (if the service has non-obvious behaviors)
- testing.md (test patterns and coverage expectations)
- observability.md (logging conventions)
Flag any service missing a standard rule file that other services have.

### 7. Connection to Learnings
- Patterns that originated from cross-project learnings should reference their source:
  "See private/learnings.md#anchor"
- If a learning has graduated to a permanent rule, the learning entry should note this
- Check: are there promoted learnings that should have been captured as rules but weren't?

### 8. Budget
- No explicit line limit, but rules files should be focused
- If a rules file exceeds ~200 lines, consider splitting by concern
- Mental models should be especially concise — capture the insight, not a tutorial

## Common Issues

1. **Aspirational content:** Describes patterns the team wants to adopt, not current state
2. **Obvious information:** Documents things Claude can infer from reading the code
3. **Missing examples:** Rules without concrete examples are hard to apply correctly
4. **Cross-service leakage:** Backend testing patterns duplicated in frontend testing.md
5. **Stale patterns:** Code evolved but rules file still describes old approach
6. **Orphaned from learnings:** A learning was promoted to a rule but the learning entry
   wasn't updated to note "graduated to rule"

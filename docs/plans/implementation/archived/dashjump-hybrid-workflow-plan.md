# dashJump.gg Hybrid Workflow Implementation Plan

## Overview

This document is a complete implementation specification for a Claude Code session to set up dashJump.gg's AI-driven development workflow. It weaves together three inputs:

1. **Subagent architecture** — Design and infrastructure specialist agents from our prior research
2. **Product Kata methodology** — Melissa Perri's outcome-driven experiment framework adapted for solo development
3. **Hybrid workflow structure** — Best ideas cherry-picked from GitHub Spec-Kit, BMAD Method, and AWS AI-DLC

The goal: a git-committed, markdown-based workflow system that persists across machines, is easy for subagents to consume without context bloat, and keeps Product Kata's outcome-driven experiment cycles at the center of every decision.

---

## Part 1: What We're Drawing From Each Framework

### From GitHub Spec-Kit
- **Constitution concept** → becomes our root `CLAUDE.md` with cross-cutting project principles
- **Numbered feature directories** → adapted as numbered experiment directories in `private/product/experiments/`
- **All-markdown-in-git persistence** → our entire workflow state is committed, never in ephemeral storage
- **Functional-first specs** (what and why before how) → aligns perfectly with Product Kata's problem-first philosophy

### From BMAD Method
- **Context sharding** → atomic task files with only the context needed for that unit of work (BMAD's single most transferable idea — claimed 90% token savings)
- **Analyst agent's assumption-challenging** → adapted into our Product Kata experiment review process
- **PM agent's scope discipline** → "zero tolerance for scope creep" maps to Perri's "deprioritize ruthlessly"
- **Quick Flow for small changes** → skip ceremony for bugs, tweaks, and small improvements
- **Story file structure** → adapted as experiment step files with embedded acceptance criteria

### From AWS AI-DLC
- **Adaptive depth selection** → AI analyzes complexity and recommends which workflow stages to execute or skip
- **Structured questions-in-files** → requirements gathered through markdown files, not ephemeral chat
- **State tracking file** (`aidlc-state.md`) → becomes our `private/CONTEXT.md` for machine-switching
- **Checkpoint artifacts** → each workflow phase produces a reviewable artifact

### Discarded from all three
- Enterprise governance gates and approval workflows
- Multi-agent ceremony (BMAD's 21 agents, AI-DLC's Mob rituals)
- Package/CLI dependencies (no `specify-cli`, no `npx bmad-method install`)
- Team-oriented rituals with no solo equivalent
- Comprehensive upfront planning that conflicts with discovery-driven development

---

## Part 2: Workflow Layers with Product Kata Integration

The workflow has four layers. The **strategic layer** and **experiment layer** are where Product Kata lives. The **specification layer** bridges discovery to implementation. The **implementation layer** is where subagents do the work.

### Layer 1: Strategic Layer (`private/product/strategy/`)

This is the persistent home for Perri's strategy deployment framework. These files change infrequently and provide the "why" for everything below.

#### Files

**`private/product/strategy/vision.md`**
```markdown
# dashJump.gg Vision

## Mission (why we exist)
[Why dashJump.gg exists — the problem space we serve]

## Vision (where we're going, 5-10 year horizon)
[Qualitative, aspirational destination for esports analytics]

## Current Strategic Intent (1-2 year focus)
[ONE outcome-oriented goal framed as business value]
[Must fit one of: increase revenue | protect revenue | reduce costs | avoid costs]

## Product Initiative (problem to solve)
[The product problem that, if solved, achieves the strategic intent]
[Framed as a PROBLEM, not a solution]

## Last reviewed: [date]
```

This file should rarely change. It's the North Star that prevents drift.

**`private/product/strategy/current-options.md`**
```markdown
# Active Options (Bets)

Options are potential solutions to our product initiative. These are hypotheses
to validate, NOT commitments to build.

## Active Options
| # | Option | Status | Linked Experiment |
|---|--------|--------|-------------------|
| 1 | [Description] | exploring / testing / validated / abandoned | experiments/NNN/ |

## Parking Lot
Options we've identified but aren't pursuing yet.
- [Option description] — Why parked: [reason]

## Abandoned Options
Options we tested and moved on from, with learnings.
- [Option] — Learned: [what we discovered]
```

### Layer 2: Experiment Layer (`private/product/experiments/`)

**This is the heart of the system — where Product Kata's experiment cycle lives as structured, versioned markdown.** Each experiment gets a numbered directory. This is where Spec-Kit's numbered directories meet Product Kata's one-week cycles.

#### The Product Kata Cycle as Files

Each experiment follows the complete Kata structure from our instructions file:

**`private/product/experiments/NNN-experiment-name/kata.md`**
```markdown
# Experiment NNN: [Name]

## Product Kata

### Direction/Challenge
[The lofty goal we're working toward — references strategic intent]

### Target Condition
[Specific, measurable near-term goal]
[Must be quantified — "coaches use dashJump weekly" not "coaches like dashJump"]

### Current Condition
[Where we are now, quantified with data]
[Established through measurement, not assumption]

### Obstacles
[List of obstacles preventing us from reaching target condition]

### Current Obstacle (ONE)
[The single obstacle we're addressing now]
[Product Kata requires focusing on ONE at a time]

## Experiment Steps

### Step 1: [Description]
- **Action:** [Specific action with measurement criteria]
- **Expected:** [What we believe will happen and why]
- **Time-box:** [≤ 1 week]
- **How we'll measure:** [Concrete measurement approach]
- **Learned:** [Filled after step completes]
- **Date completed:** [date]

### Step 2: [Description]
[Same structure — only created after Step 1 completes and we decide to continue]

## Coaching Kata Check-in
[Answer these five questions at each checkpoint]
1. What is the target condition? → [answer]
2. What is the actual condition now? → [answer]
3. What obstacles prevent reaching target? Which ONE now? → [answer]
4. What is your next step? What do you expect? → [answer]
5. When can we see what we learned? → [answer]

## Experiment Type
[concierge | wizard-of-oz | concept-test | build-to-learn | build-to-ship]

## Cost of Delay Assessment
- **Urgency:** [Does not doing this prevent validation/learning?]
- **Value:** [Does this solve the strongest pain point identified?]
- **Priority:** [high-high: build now | high-low: question urgency | low-high: schedule | low-low: deprioritize]

## North Star (before building anything)
- **Problem being solved:** [validated how?]
- **Proposed approach:** [solution direction]
- **Success factors:** [what must be true]
- **Expected outcomes:** [measurable results]
- **How we'll know it worked:** [specific criteria]

## Definition of Done (Experiment-Level Outcome)
[The outcome that determines whether this experiment succeeded]
[Based on achieving OUTCOMES, not shipping V1]
[e.g., "2+ coaches use this weekly to make decisions they couldn't before"]
[Note: individual features have their own Success Criteria in their specs]

## Linked Spec (when building)
- **Spec:** [private/specs/NNN-feature-name.md — added when experiment moves to building]
- **Spec status:** [draft | ready | in-progress | completed | validated]

## Status: [discovery | active-experiment | validating | achieved | abandoned]
## Created: [date]
## Last updated: [date]
```

#### Experiment Sequencing

Following Perri's validation progression:

```
Discovery Interview → Concierge Test → Concept Test → Wizard of Oz → Build Real Solution
```

**Critical rule from Product Kata: Do NOT move to the next experiment or feature until the current one achieves its target condition.** Only 15-20% of product managers iterate after shipping. This file structure enforces iteration by requiring `Status: achieved` before starting the next numbered experiment.

#### Learnings Archive

**`private/product/experiments/NNN-experiment-name/learnings.md`**
```markdown
# Learnings from Experiment NNN

## Summary
[1-2 sentence summary of what we learned]

## Expected vs Actual
| What we expected | What actually happened |
|-----------------|----------------------|
| [expectation] | [reality] |

## Key Insights
- [Insight that changes how we think about the problem]

## Impact on Strategy
- [How this affects our strategic intent, product initiative, or options]

## What We'd Do Differently
- [Process improvements for future experiments]

## Date completed: [date]
```

### Layer 3: Specification Layer (`private/specs/`)

Specs are created ONLY when an experiment step requires building something. They bridge the "what to learn" (experiment layer) to "what to build" (implementation layer). Not every experiment needs a spec — concierge tests and interviews don't.

**`private/specs/NNN-feature-name.md`**
```markdown
# Spec: [Feature Name]

## Links
- **Experiment:** experiments/NNN-experiment-name/kata.md
- **Option:** strategy/current-options.md#N
- **Outcome target:** [What success looks like — from experiment's target condition]
- **Created:** [date]
- **Last updated:** [date]

## Problem Statement
[What problem this solves, validated through which experiment step]
[Why coaches need this — in their words if possible, reference coach interview]

## Assumptions
What MUST be true for this feature to succeed:
- **Data:** [e.g., "game_clock is always reconciled correctly; demo_time is NOT a substitute
  (see private/learnings.md#demo-timeline-offset)"]
- **Technical:** [e.g., "Frontend can render 500+ data points without performance degradation"]
- **Architectural:** [e.g., "Large match data stored in S3, not JSONB
  (see private/learnings.md#s3-storage-solves-jsonb-bottleneck)"]
- **User behavior:** [e.g., "Coaches prioritize wave/lane analytics over damage breakdowns
  (see private/learnings.md#wave-priority-tracking)"]

## Data Requirements & Transformations
[What data flows through this feature — end-to-end from parser to UI]

### Input Data
- **Source:** [Parser output | Backend transform | S3 storage | External API]
- **Format:** [JSON structure, array shape, time-series, etc.]
- **Volume:** [Approximate size — e.g., "~15 MB per match"]
- **Example:** [Actual data structure snippet]

### Transformations
- **Parser → Raw JSON:** [What parser produces for this feature]
- **Raw → Backend storage:** [Aggregation, normalization, what changes shape]
- **Storage → API response:** [Filtering, derived fields, time-range selection]
- **API → Frontend:** [Any client-side transformation needed]

### Output Data (what UI receives)
- **Shape:** [JSON structure or TypeScript interface]
- **Time range:** [Full match | phase-specific | user-selectable]
- **Example:** [Final shape the frontend renders]

## Related Docs
- [.claude/rules/parser/parser-mental-model.md — if timeline or game_clock involved]
- [.claude/rules/backend/backend-mental-model.md — if data transformation-heavy]
- [private/learnings.md#[relevant-learning] — cite specific learnings that apply]
- [Other specs this builds on — e.g., "requires wave detection from Spec 003"]
- [Coach interview notes that informed this — e.g., private/product/experiments/001/interviews.md]
- [Service CLAUDE.md files for stack conventions — e.g., backend/CLAUDE.md]

## Risk
[LOW: proceed directly | MEDIUM: validate at shard boundaries | HIGH: spike/POC first]
[One sentence explaining why — e.g., "HIGH — parser output structure uncertain, blocks everything downstream"]

## Functional Requirements (What and Why — NOT How)
[Following Spec-Kit's functional-first principle]
1. [User-facing requirement]
2. [User-facing requirement]

## Acceptance Criteria
- [ ] [Measurable criterion tied to experiment outcome]
- [ ] [Measurable criterion]
- [ ] [All assumptions verified or documented as acceptable risks]
- [ ] [Data transformations validated end-to-end (parser → backend → frontend)]

## Out of Scope
[Explicitly what we're NOT building — BMAD's scope discipline]

## Technical Notes
[Only if genuine constraints — DDD layer, API patterns, etc.]
[Delete this section if empty — don't fill with speculation]

## Success Criteria
[Feature-specific outcomes that define when THIS spec is complete]
[Distinct from project-level Definition of Done which applies to all work]
- [Outcome criterion — e.g., "Coach references wave priority in post-match analysis"]
- [Behavioral criterion — e.g., "Visualization loads in <500ms"]
- [Validation criterion — e.g., "[redacted coach] confirms data matches their expectations"]

## Task Shards
[Following BMAD's context sharding — each shard is one atomic unit of work
that a subagent can execute independently with minimal context]

### Shard 1: [Name]
- **Assigned agent:** [rust-parser | backend-python | frontend-react | etc.]
- **Files to modify:** [specific paths]
- **Context needed:** [only what's relevant — data contract, acceptance criteria subset]
- **Done when:** [specific, testable condition]
- **Dependencies:** [None | "Shard N must be complete"]
- **Verify before proceeding:** [What Steven checks before next shard starts —
  e.g., "Run parser on 3 sample matches, confirm output shape matches Data Requirements"]

### Shard 2: [Name]
- **Assigned agent:** [agent name]
- **Files to modify:** [specific paths]
- **Context needed:** [Shard 1 output shape, relevant mental model docs]
- **Done when:** [specific, testable condition]
- **Dependencies:** [Shard 1 — verified]
- **Verify before proceeding:** [What Steven checks — e.g., "Test API endpoint with
  real match data, confirm response shape matches spec, response <500ms"]

### Shard 3: [Name]
[Same structure — verification here is end-to-end: full pipeline works,
visualization renders correctly, matches coach expectations]
```

**Key design decisions in this template:**

**Verification is inline, not a separate section.** Each shard's `Verify before proceeding:` field keeps Steven in the loop at every service boundary. This replaces formal checkpoint sections — same gating, zero extra overhead.

**Success Criteria is feature-specific.** This is distinct from the project-level Definition of Done (in root CLAUDE.md) that applies to all work. Success criteria answers "when is *this particular feature* done?" while DoD answers "what standards must *all* work meet?"

**Risk is a single field.** LOW/MEDIUM/HIGH with one sentence. This informs whether to spike first (HIGH), add extra verification (MEDIUM), or proceed directly (LOW). No per-service complexity ratings.

**Task shards are the critical bridge to subagents.** Each shard contains ONLY the context needed for that unit of work — not the full spec, not the full experiment. This is BMAD's most valuable contribution: 90% token savings by giving agents atomic, self-contained work packages.

### Layer 4: Implementation Layer (`.claude/` + code)

This is where subagents, skills, and hooks live. The implementation layer doesn't contain product strategy — it contains the agent configuration and tooling that executes work defined by the layers above.

---

## Part 3: Subagent Architecture

### Design Pipeline Subagents

Based on our prior research, design is a top priority given the current lack of brand guidelines. The design pipeline has two phases: brand establishment (one-time) and ongoing UI implementation.

#### Phase 1: Brand Establishment (Skill, not Subagent)

Brand guidelines should be a **skill** (`.claude/skills/dashjump-brand/SKILL.md`), not a subagent. Skills auto-load when relevant context is detected, making them persistent domain knowledge rather than a task executor. This is more efficient — the brand guidelines inform every design decision without consuming a separate agent context.

**`.claude/skills/dashjump-brand/SKILL.md`**
```markdown
---
name: dashjump-brand
description: dashJump.gg brand identity and design system. Apply these guidelines
  for any frontend visual work, component styling, or UI design decisions.
---

# dashJump.gg Brand Identity

## Brand Positioning
[Esports analytics for competitive coaches — data-dense but clean]
[Aesthetic direction: Bloomberg Terminal meets gaming culture]

## Color System
[Primary palette — defined after brand discovery session]
[Semantic colors — success, warning, error, info]
[Data visualization palette — 12+ distinct colors for charts]
[Dark mode primary (esports audience expects dark themes)]

## Typography
[Display font — for headings, hero text]
[Body font — for readable analytics content]
[Monospace font — for data tables, code, stat displays]
[Font scale and weights]

## Spacing & Layout
[Base unit, spacing scale]
[Grid system]
[Breakpoints]

## Component Patterns
[Card patterns for match data]
[Data table conventions]
[Chart/visualization containers]
[Navigation patterns]

## Voice & Tone
[How dashJump communicates — technical but accessible]
```

This file gets populated through a dedicated brand discovery session (which is itself an experiment in the Product Kata sense — "Can we establish a brand identity that resonates with our coach audience?").

#### Security & Compliance Skill

Like brand guidelines, security and compliance requirements are **domain knowledge that multiple agents need**, not a task for one agent. The code-reviewer references this skill explicitly, and service agents consult it when handling auth, user data, or external API integration.

**`.claude/skills/dashjump-compliance/SKILL.md`**
```markdown
---
name: dashjump-compliance
description: Security requirements, data handling policies, and compliance
  guidelines for dashJump.gg. Referenced by code-reviewer for security audits
  and by service agents when handling user data or authentication.
---

# dashJump.gg Security & Compliance

## Compliance Tier Assessment
dashJump.gg is a pre-revenue B2B tool targeting individual esports coaches.
Current compliance posture: Tier 1 (foundational security) + Tier 2 (user
data protection). SOC2 certification is NOT needed at this stage — revisit
only if selling to esports organizations with formal procurement processes.

## Tier 1: Application Security (enforce now)

### Authentication & Sessions
- Steam OpenID 2.0: validate return_to URL, verify claimed_id against Steam
- Session tokens: cryptographically random, HttpOnly, Secure, SameSite=Lax
- CSRF protection on all state-changing endpoints
- Session expiration: define max lifetime and idle timeout
- Logout must invalidate server-side session, not just clear cookies

### Input Validation
- All database queries: parameterized (SQLAlchemy models, no string interpolation)
- API input: Pydantic validation on every endpoint (already in conventions)
- File uploads (if any): validate type, size limits, no path traversal
- Parser input: treat all replay file data as untrusted

### Secrets Management
- NEVER commit API keys, DB credentials, session secrets, or tokens to git
- Environment variables for local dev
- AWS Secrets Manager or Parameter Store for production (when deployed)
- Rotate secrets if ever exposed

### Transport Security
- HTTPS everywhere in production (no exceptions)
- HSTS header in production
- No mixed content (HTTP resources on HTTPS pages)

### Security Headers (backend must set these)
- Content-Security-Policy: restrict script/style sources
- X-Frame-Options: DENY (prevent clickjacking)
- X-Content-Type-Options: nosniff
- Referrer-Policy: strict-origin-when-cross-origin
- CORS: allowlist specific origins, not wildcard

### Dependency Security
- Run `npm audit`, `pip audit`, `cargo audit` in CI
- Flag and address critical/high vulnerabilities before merge
- Pin dependency versions (lockfiles committed)

## Tier 2: Data Protection (enforce before scaling)

### GDPR Applicability
Applies if ANY coaches or their players are in the EU (very likely in esports).
At current scale, compliance is straightforward:

Required:
- Privacy policy: what data collected, why, how long retained, how to delete
- Right to deletion: ability to delete a user's data on request
- Data inventory: document what you store, where, and retention period
- Lawful basis: legitimate interest (analytics service) or consent

Not required at this scale:
- Data Protection Officer
- Data Processing Agreements
- Formal Data Protection Impact Assessment

### Data Handling Rules
- User data: Steam ID, match history, analytics preferences
- Match data: replay files, parsed game events, derived statistics
- Log what data is collected and where it's stored (data inventory doc)
- Define retention periods (how long do we keep parsed match data?)
- Never store data you don't need

### Steam Terms of Service
- Verify compliance with Steam Web API Terms of Use
- Respect rate limits on Steam API calls
- Don't store or redistribute Steam user data beyond what's needed
- Display required Steam attributions if applicable

### Rate Limiting
- API endpoints: implement rate limiting before public access
- Respect upstream rate limits (Steam API, any third-party services)
- Log rate limit hits for monitoring

## Tier 3: Future (when scaling or seeking investment)
These are NOT needed now. Document here so they're not forgotten:
- SOC2 Type II certification (only for enterprise/org sales)
- Formal incident response plan
- Data breach notification procedures
- Comprehensive audit logging
- Backup and disaster recovery strategy
- Penetration testing

## For Code Reviewers
Flag as CRITICAL:
- Unparameterized database queries
- Missing auth checks on endpoints
- Secrets in code or config files
- Missing CSRF protection on state-changing routes
- User data logged at INFO level or higher

Flag as WARNING:
- Missing security headers
- Dependencies with known vulnerabilities
- Missing rate limiting on public endpoints
- Overly permissive CORS configuration
```

#### Phase 2: Design Subagent

**`.claude/agents/dashjump-designer.md`**
```markdown
---
name: dashjump-designer
description: UI/UX design specialist for dashJump.gg. Use for any frontend
  visual work including React components, page layouts, data visualizations,
  and design system maintenance. Applies brand guidelines automatically.
tools: Read, Write, Edit, Bash, Glob, Grep
model: sonnet
skills: dashjump-brand
---

You are a senior UI/UX designer for an esports analytics platform.

## Plugins Available
- `frontend-design` — production-grade UI design patterns, avoids generic AI aesthetics

## Design Principles
- Data-dense but clean — every pixel earns its place
- Story-first visualizations (tell a story, don't just display data)
- Game-phase aware layouts (laning, mid-game, late game)
- Dark mode primary, light mode secondary
- Accessible to colorblind users (critical for data viz)

## Technical Stack
- React + TypeScript
- Tailwind CSS with custom design tokens from brand skill
- shadcn/ui as component foundation
- Recharts for data visualization
- Follow conventions in frontend/CLAUDE.md

## When Creating Components
1. Check if brand skill has relevant patterns
2. Use Tailwind utilities, not arbitrary CSS values
3. Include responsive breakpoints
4. Add loading and error states
5. Consider dark/light theme variants
```

#### MCP Server for Design

Install the **tailwindcss-mcp-server** for color palette generation and CSS-to-Tailwind conversion. This gives the design subagent concrete tools beyond code generation.

### Infrastructure Subagent (Deferred)

The infrastructure subagent plan has been separated into `private/plans/infra-subagent-plan-deferred.md`. It's ready to activate once open questions around Kubernetes, S3/Parquet storage architecture, and Terraform scope are resolved through Product Kata experiments. The full subagent definition, MCP server setup, and community skills evaluation are preserved there.

**Activation trigger:** At least two of these questions have clear answers: K8s vs ECS vs simpler option, S3/Parquet migration approach, Terraform scope, monthly cost target.

### Service-Specific Subagents

**`.claude/agents/backend-python.md`**
```markdown
---
name: backend-python
description: Python/FastAPI backend specialist. Use for API endpoints, database
  models, Pydantic schemas, use cases, domain services, backend business logic,
  and all backend tests.
tools: Read, Write, Edit, Bash, Glob, Grep
model: sonnet
---

You are a Python/FastAPI backend expert for an esports analytics platform.

## Plugins Available
- `pyright-lsp` — real-time Python type checking and diagnostics

Follow project conventions in backend/CLAUDE.md:
- Domain-Driven Design layers (API → Application → Domain → Infra)
- Pydantic v2 with strict validation
- Async SQLAlchemy patterns
- Dependency injection via FastAPI Depends
- Google-style docstrings

## Before Starting Work
Check private/learnings-index.md for applicable learnings when working on:
- Timeline or game_clock logic (demo offset reconciliation)
- Storage decisions (S3 vs JSONB patterns)
- Data transformation pipelines
- Features informed by coach feedback
Also check .claude/rules/backend/backend-mental-model.md for architecture constraints.

## Testing (integrated — no separate test agent)
Tests are YOUR responsibility, written alongside implementation code:
- Tests mirror app/ structure (tests/api/, tests/domain/, etc.)
- Domain tests: no mocking of domain internals, test business rules
- Application tests: mock repositories and external services
- Error path tests: every error category needs a test (400, 404, 500, 502)
- Coverage targets: Domain 90%+, Application 80%+, Infrastructure critical paths
- Run pytest after changes to verify nothing breaks
- See .claude/rules/backend/testing.md for patterns

## Observability (integrated — no separate observability agent)
Instrument code as you write it:
- Structured logging: include correlation IDs, match_id, duration
- Log levels: DEBUG for dev, INFO for operations, WARNING for recovered issues, ERROR for failures
- Never log secrets, tokens, PII, or full request bodies
- See .claude/rules/backend/observability.md for conventions

## Shared File Rules
- Do NOT write to strategy files (vision.md, current-options.md) or learnings-index.md
- If you discover a cross-project pattern, append to private/learnings.md ## Drafts section only
- Format: `### [Draft] [Topic] — [agent: backend-python, date: YYYY-MM-DD]\n[Finding]`
```

**`.claude/agents/frontend-react.md`**
```markdown
---
name: frontend-react
description: React/TypeScript frontend specialist. Use for components, hooks,
  state management, data fetching, frontend architecture, and all frontend tests.
tools: Read, Write, Edit, Bash, Glob, Grep
model: sonnet
skills: dashjump-brand
---

You are a React/TypeScript frontend expert for an esports analytics platform.

## Plugins Available
- `typescript-lsp` — real-time TypeScript type checking and diagnostics
- `frontend-design` — production-grade UI patterns, avoids generic AI aesthetics

Follow project conventions in frontend/CLAUDE.md:
- TypeScript strict mode
- Tailwind CSS with brand design tokens
- Component composition over prop drilling
- Error boundaries for graceful degradation

## Before Starting Work
Check private/learnings-index.md for applicable learnings when working on:
- Timeline visualizations (demo offset affects display)
- Data-heavy components (coach-validated priorities)
- Match analysis features (wave priority > kill data)

## Testing (integrated — no separate test agent)
Tests are YOUR responsibility, written alongside components:
- React Testing Library with Vitest Browser for component tests
- Test hierarchy: Critical user paths → Error handling → Edge cases → Accessibility
- Every error state must test: error display, retry functionality, recovery
- Mock external dependencies (fetch, images, env vars)
- Focus on critical user paths over arbitrary coverage numbers
- See .claude/rules/frontend/testing.md for patterns

## Observability (integrated — no separate observability agent)
- console.error() for caught exceptions with sanitized context
- console.warn() for recoverable issues and unexpected states
- Never log auth tokens, session data, PII, or full API responses
- Error boundaries must log component stack traces
- See .claude/rules/frontend/observability.md for conventions

## Shared File Rules
- Do NOT write to strategy files (vision.md, current-options.md) or learnings-index.md
- If you discover a cross-project pattern, append to private/learnings.md ## Drafts section only
- Format: `### [Draft] [Topic] — [agent: frontend-react, date: YYYY-MM-DD]\n[Finding]`
```
```markdown
---
name: rust-parser
description: Rust/Axum parser service specialist. Use for replay parsing logic,
  Axum endpoints, data extraction, Rust-specific optimizations, and parser tests.
tools: Read, Write, Edit, Bash, Glob, Grep
model: sonnet
---

You are a Rust systems engineer working on a replay parser service.

## Plugins Available
- `rust-analyzer-lsp` — real-time Rust analysis, type checking, and diagnostics

Follow project conventions in parser/CLAUDE.md:
- Result<T, E> everywhere — no panics
- Custom error types with thiserror
- Axum for HTTP layer

## Before Starting Work
ALWAYS check .claude/rules/parser/parser-mental-model.md before modifying parser logic.
Check private/learnings-index.md for cross-project learnings, especially:
- Demo timeline offset (frame reconciliation, game_clock vs demo_time)
- Any timeline or position data handling

## Testing (integrated)
- Write tests alongside implementation using Rust's built-in test framework
- Test error paths: malformed replay data, missing fields, corrupt files
- Integration tests for Axum endpoints
- See .claude/rules/parser/ for patterns

## Observability (integrated)
- Tracing crate for structured logging
- Instrument parse operations with duration and data size
- See .claude/rules/parser/observability.md for conventions

## Shared File Rules
- Do NOT write to strategy files (vision.md, current-options.md) or learnings-index.md
- If you discover a cross-project pattern, append to private/learnings.md ## Drafts section only
- Format: `### [Draft] [Topic] — [agent: rust-parser, date: YYYY-MM-DD]\n[Finding]`
```

### Utility Subagents

**`.claude/agents/code-reviewer.md`**
```markdown
---
name: code-reviewer
description: Code review and security specialist. Use after completing
  implementation work to review changes for bugs, security vulnerabilities,
  convention violations, compliance concerns, and architectural issues.
  Read-only — does not modify files.
tools: Read, Bash, Glob, Grep
model: sonnet
skills: dashjump-compliance
---

You are a senior code reviewer and application security specialist.

## Plugins Available
- `code-review` — enhanced AI code review with specialized agents
- `security-guidance` — security analysis and vulnerability detection
- `pr-review-toolkit` — PR review workflow tools

## Review Checklist (in priority order)

### 1. Security (always check)
- Authentication: session handling, CSRF protection, token security
- Authorization: access control on every endpoint, no IDOR vulnerabilities
- Input validation: parameterized queries (no string interpolation in SQL),
  sanitized user input, type validation on API boundaries
- Secrets: no API keys, credentials, or tokens in code or git history
- Dependencies: flag known-vulnerable package versions
- Headers: CORS policy, CSP, X-Frame-Options, X-Content-Type-Options
- Steam integration: verify OpenID 2.0 flow follows Steam's ToS
- See .claude/skills/dashjump-compliance/SKILL.md for full security checklist

### 2. Convention Violations
- Check relevant CLAUDE.md files for the service being modified
- DDD layer boundaries (backend): no domain logic in API layer
- Error handling patterns per service

### 3. Logic Errors and Edge Cases
- Null/undefined handling
- Race conditions in async code
- Boundary conditions (empty arrays, max values, missing data)

### 4. Missing Error Handling
- Every external call (DB, API, parser) must have error handling
- User-facing errors must be safe (no stack traces, internal paths)

### 5. Test Coverage Gaps
- New code paths should have corresponding tests
- Error paths should be tested, not just happy paths

### 6. Quality Thresholds (warn/flag)
- Domain layer test coverage: warn below 85%, flag below 70%
- Function cyclomatic complexity: warn above 10, flag above 15
- Function length: warn above 40 lines, flag above 50 without documented justification
- Zero unparameterized SQL queries (no exceptions)

### 7. Knowledge Management
- If code touches timeline/game_clock: verify spec cites demo-timeline-offset learning
- If code touches storage/large data: verify spec cites S3 storage learning
- If code implements coach-requested feature: verify spec cites coach validation source
- If code has non-obvious patterns: verify code comments link to mental models
- Check private/learnings-index.md for applicable learnings in the affected area

## Output Format
- CRITICAL: Must fix before merge (security issues, data exposure, broken auth)
- WARNING: Should fix, not blocking (missing tests, convention drift)
- SUGGESTION: Nice to have (naming, structure, minor optimization)

Only report issues with >80% confidence. Do not modify any files.
```

**`.claude/agents/spec-writer.md`**
```markdown
---
name: spec-writer
description: Documentation and specification writer. Use for writing experiment
  kata files, learnings documents, feature specs, coach-facing materials,
  and updating product strategy docs. Focused on clarity and structure.
tools: Read, Write, Edit, Glob, Grep
model: sonnet
---

You are a technical writer and product analyst for an esports analytics platform.

## Responsibilities
- Write and update Product Kata experiment files (kata.md, learnings.md)
- Draft feature specifications with task shards
- Update strategy documents (vision.md, current-options.md)
- Write CONTEXT.md for machine-switching
- Draft coach-facing materials (interview guides, demo scripts)
- Compliance documentation (privacy policy, data inventory, Steam ToS notes)
- Consolidate learnings: promote drafts from private/learnings.md ## Drafts section,
  deduplicate, prune completed items, and update private/learnings-index.md

## Shared File Ownership (you are the sole writer for these)
- `private/product/strategy/vision.md` — strategic direction
- `private/product/strategy/current-options.md` — active bets and outcomes
- `private/learnings-index.md` — only updated during consolidation
- `private/learnings.md` (above ## Drafts) — promoted, vetted learnings only
Note: Service agents may append raw findings to the ## Drafts section of learnings.md.
You are responsible for reviewing, promoting, or discarding those drafts.

## Before Writing Any Spec
1. Check private/learnings-index.md for cross-project learnings relevant to the feature
2. Cite applicable learnings in the spec's Assumptions section with links
3. Link to service mental models in Related Docs if the spec touches that service
4. Every data assumption should reference its source (learning, mental model, or interview)

## Writing Principles
- Outcome-focused: every document connects back to a measurable goal
- Concise: specs under 5,000 tokens, task shards under 2,000 tokens
- Structured: follow the templates in private/product/
- Honest: record what was actually learned, not what we hoped to learn

## Product Kata Awareness
- Experiments must have measurable target conditions
- Steps must be time-boxed to ≤ 1 week
- Experiment-level Definition of Done = outcome achieved, not feature shipped
- Feature-level Success Criteria = specific to each spec, outcome-tied
- Reference the framework in private/product/strategy/

## Do NOT
- Make implementation decisions (that's for service agents)
- Write code (that's for service agents)
- Skip the North Star document before any feature spec

## End-of-Task Reminder
After completing any consolidation, revision, or spec work, end with:
"Note: [N] findings flagged for Steven-owned files (agents/commands/skills/CLAUDE.md).
Run the dashjump-context-audit skill for details."
Omit this line if no Steven-owned findings were flagged.
```

### Quality Assurance Subagents

**`.claude/agents/e2e-playwright.md`**
```markdown
---
name: e2e-playwright
description: End-to-end test specialist using Playwright. Use for writing and
  maintaining cross-service E2E tests that span the full user flow (frontend →
  backend → parser). Covers user journeys that no single service agent can test.
tools: Read, Write, Edit, Bash, Glob, Grep
model: sonnet
---

You are a QA engineer specializing in end-to-end testing with Playwright.

## Plugins Available
- `playwright` — Playwright integration for browser testing

## Context
- App: esports analytics platform (Deadlock game)
- Stack: React frontend, FastAPI backend, Rust parser, PostgreSQL
- Dev environment: Docker Compose (all services start with one command)
- Playwright is already installed

## Core User Flows to Test
1. Login: Home page → Steam OpenID redirect → callback → match history
2. Match history: View list → select a match → navigate to analysis
3. Match analysis: Sankey tabs → minimap interaction → player/lane cards → timeline
4. Error states: Service down, invalid match, network failure

## Testing Principles
- ALWAYS use semantic locators: getByRole(), getByText(), getByLabel()
- NEVER use CSS selectors or data-testid unless no semantic alternative exists
- Semantic locators survive visual redesigns — CSS selectors don't
- Each test should be independent (no test ordering dependencies)
- Test the user's perspective, not implementation details
- Include both happy paths and key error scenarios

## Test Structure
- tests/e2e/ at project root (spans all services)
- Organize by user journey, not by page
- Include setup/teardown for test data
- Use Playwright's built-in waiting (avoid arbitrary timeouts)

## When Writing Tests
1. Start from the user's goal ("I want to analyze my last match")
2. Write the flow as the user would experience it
3. Assert on what the user would see, not internal state
4. Add error scenario variants (what if the match doesn't load?)
5. Keep tests under 30 lines — if longer, the flow might need splitting
```

**`.claude/agents/test-auditor.md`**
```markdown
---
name: test-auditor
description: Test suite auditor. Use periodically (monthly or before releases)
  to scan all test files across services for coverage gaps, missing error path
  tests, stale tests, and pattern violations. Read-only — does not modify files.
tools: Read, Bash, Glob, Grep
model: sonnet
---

You are a QA architect performing a systematic test suite audit.

## Audit Scope
Scan test files across all three services:
- Frontend: tests using Vitest + React Testing Library + Playwright browser
- Backend: tests using pytest, organized by DDD layer
- Parser: Rust tests using built-in test framework

## What to Check

### Coverage Gaps
- Components/endpoints/functions with no corresponding test file
- Error paths without tests (every HTTP status code, every exception type)
- Edge cases: empty data, null values, max-size inputs, concurrent access
- Integration boundaries: API responses, database queries, parser output

### Test Quality Issues
- Tests that test implementation rather than behavior
- Missing assertions (tests that "pass" by not checking anything)
- Overly broad mocks that hide real bugs
- Hardcoded test data that could mask boundary conditions

### Staleness
- Test files that reference deleted components or renamed functions
- Skipped/disabled tests with no explanation
- Tests that always pass regardless of implementation changes

### Pattern Compliance
- Backend: follows DDD test layering (domain no mocks, app mocks repos)?
- Frontend: uses semantic queries (getByRole) over CSS selectors?
- Error handling: tested per .claude/rules/{service}/testing.md patterns?

### Flaky Test Detection
- Tests with inconsistent pass/fail across multiple runs
- Common causes to check: timing-dependent assertions, shared test data between tests,
  race conditions in async code, environment-dependent behavior
- Recommendation for each: quarantine (skip with TODO), fix (root cause identified), or delete

### Test Pyramid Health
Report the current ratio across the suite:
- Unit tests (isolated, no external dependencies)
- Integration tests (service boundaries, database, external calls)
- E2E tests (full user flows via Playwright)
- Flag if E2E tests exceed 20% of total (pyramid inversion risk)
- Flag if any service has zero unit tests

### Known Gaps (current state)
- Parser: 0 tests — flag all critical parsing logic as untested
- Frontend: 53 tests across 5 files — check for untested components
- Backend: 37 tests across 10 files — verify DDD layer coverage

## Output Format
### CRITICAL GAPS (untested critical paths)
- [File/function]: [What's missing] — Risk: [potential impact]

### COVERAGE OPPORTUNITIES (important but not critical)
- [Area]: [What to test] — Effort: [low/medium/high]

### QUALITY ISSUES (existing tests that need improvement)
- [Test file]: [Problem] — Fix: [recommendation]

### METRICS
- Estimated coverage by service and layer
- Test pyramid ratio: unit% / integration% / E2E%
- Number of untested error paths found
- Number of stale/skipped tests found
- Number of suspected flaky tests

Do not modify any files. Output a prioritized action list.
```

### Subagent Design Rationale

**Why testing and observability are integrated into service agents:**

Testing and observability are best done by the agent that writes the code. Each service agent has full context of the implementation it just created — edge cases it considered, error paths it handled, what's worth instrumenting. A separate testing agent would need to re-read every file and re-learn the stack-specific patterns (pytest for backend, Vitest/RTL for frontend, Rust's built-in framework for parser), duplicating context for no gain.

The project already has comprehensive testing and observability documentation per service in the `.claude/rules/` hierarchy that each service agent naturally reads when working in its subdirectory.

**Why E2E tests DO get a separate agent:**

End-to-end tests span the full stack — frontend → backend → parser → database. No single service agent has the cross-cutting view needed. The E2E agent understands user journeys, not service internals. It uses Playwright's semantic locators (which survive visual redesigns) and tests from the user's perspective. Current user flow is stable enough to start with smoke tests: home → Steam login → match history → match analysis page with Sankey tabs, minimap, player cards, and timeline.

**Why test auditing is separate from both E2E and code-reviewer:**

The test auditor is a periodic sweep (monthly or before releases), not a per-change review. It scans the *entire* test suite across all services for systemic issues: coverage gaps, stale tests, missing error paths, pattern violations. Code-reviewer catches test gaps in individual changes; the auditor catches the accumulated drift. Different cadence, different scope, different output format. Known current state: 90 tests (53 frontend, 37 backend, 0 parser) — the parser gap alone justifies the auditor.

**Why security and compliance are a skill, not a separate agent:**

Your friend's SOC2/GDPR agent makes sense for teams with formal compliance requirements. For dashJump at pre-revenue stage, compliance is a *checklist* (security headers, CSRF, parameterized queries, GDPR basics) that the code-reviewer enforces on every change and service agents follow when writing code. Making it a skill means any agent can reference it, and the code-reviewer gets it auto-loaded. This avoids adding a compliance agent that would mostly sit idle — the knowledge is distributed to where the decisions happen.

When compliance becomes a dedicated workstream (SOC2 audit prep, formal privacy impact assessments), it would graduate from a skill to a subagent. That's a Tier 3 concern.

**Why spec-writer is a separate agent:**

Writing specs and experiment documentation is a fundamentally different mode than writing code. It benefits from context isolation because:
1. The agent stays focused on clarity, structure, and outcome-alignment rather than drifting into implementation
2. Product strategy context (vision, strategic intent, experiment history) is different from codebase context
3. Coach-facing materials require a different "voice" than technical documentation
4. Task shards in specs need to be written from the *consumer's* perspective (what does a service agent need to execute this?), not the implementer's

**Complete subagent roster:**

| Agent | Domain | Status | Model |
|-------|--------|--------|-------|
| `dashjump-designer` | UI/UX, components, design system | Active | sonnet |
| `backend-python` | FastAPI, DDD, database + tests + observability | Active | sonnet |
| `frontend-react` | React, TypeScript, Tailwind + tests + observability | Active | sonnet |
| `rust-parser` | Replay parsing, Axum + tests + observability | Active | sonnet |
| `code-reviewer` | Security, compliance, conventions, bugs | Active | sonnet |
| `spec-writer` | Experiments, specs, strategy docs, coach materials | Active | sonnet |
| `e2e-playwright` | Cross-service E2E tests, user journey validation | Active (smoke tests) | sonnet |
| `test-auditor` | Periodic test suite coverage and quality audits | Active (monthly) | sonnet |
| `infra-specialist` | Terraform, Docker, CI/CD, AWS | **Deferred** — separate plan | — |

**Skills (domain knowledge, not agents):**

| Skill | Purpose | Referenced By |
|-------|---------|---------------|
| `dashjump-brand` | Brand identity, design system, visual conventions | designer, frontend-react |
| `dashjump-compliance` | Security checklist, GDPR, Steam ToS, data handling | code-reviewer, all service agents |

### When to Use Agent Teams Instead

Agent teams (research preview, February 2026) are **not recommended yet** for your workflow. They require explicit opt-in, have no session resumption, and cost N× tokens. The upgrade trigger: when you need two agents to coordinate on interface contracts (e.g., parser API changes that affect both backend and frontend simultaneously). Until then, subagents with well-structured task shards handle your parallelization needs.

---

## Part 4: Persistence and Machine-Switching

### Directory Structure

The project uses two repositories. The public repo (`dashJump.gg`) contains code, workflow tooling, and technical documentation. The private repo (`dashjump-private`) contains proprietary strategy, experiments, specs, and cross-project learnings. The private repo is mounted as a git submodule at `private/` in the public repo.

**Why two repos:** The public repo is open on GitHub. Strategy documents (vision, coach feedback, experiment outcomes, competitive bets) and feature specs (which reveal roadmap priorities) are proprietary. Separating them means subagents access everything seamlessly on the local filesystem, while only technical content is publicly visible.

```
dashJump.gg/                               # PUBLIC REPO
├── CLAUDE.md                              # ≤200 lines, universal rules + workflow pointers
├── CLAUDE.local.md                        # Gitignored: local URLs, test data, sandbox config
├── .gitmodules                            # Submodule reference to dashjump-private
├── .claude/
│   ├── settings.json                      # {"plansDirectory": "./private/plans"}
│   ├── knowledge-management.md            # Rules for capturing/organizing knowledge (5 tiers)
│   ├── commands/                          # Custom slash commands
│   │   ├── consolidate-learnings.md       # /consolidate-learnings — promote draft learnings
│   │   ├── kata-check.md                  # /kata-check — run coaching kata 5 questions
│   │   ├── new-experiment.md              # /new-experiment — scaffold experiment directory
│   │   ├── quick-fix.md                   # /quick-fix — skip ceremony for small changes
│   │   ├── switch-machine.md              # /switch-machine — write state to CONTEXT.md
│   │   ├── test-audit.md                  # /test-audit — periodic test suite audit
│   │   └── toc.md                         # /toc — scan file headers before full read
│   ├── agents/                            # Custom subagent definitions (all from Part 3)
│   │   ├── dashjump-designer.md
│   │   ├── backend-python.md
│   │   ├── frontend-react.md
│   │   ├── rust-parser.md
│   │   ├── code-reviewer.md
│   │   ├── spec-writer.md
│   │   ├── e2e-playwright.md
│   │   └── test-auditor.md
│   └── skills/                            # Domain knowledge (lazy-loaded)
│       ├── dashjump-brand/
│       │   └── SKILL.md                   # Brand guidelines (populated after discovery)
│       ├── dashjump-compliance/
│       │   └── SKILL.md                   # Security, GDPR, Steam ToS, data handling
│       └── dashjump-context-audit/
│           ├── SKILL.md                   # Context audit (all .claude/ + private/ files)
│           └── references/                # Type-specific quality rubrics
│               ├── quality-criteria-claude-md.md
│               ├── quality-criteria-agents.md
│               ├── quality-criteria-commands.md
│               ├── quality-criteria-skills.md
│               ├── quality-criteria-rules.md
│               ├── quality-criteria-knowledge.md
│               ├── context-budgets.md     # Single source of truth for all file budgets
│               └── ownership-map.md       # Who can write which files
├── .claude/rules/                         # Service standards + mental models (existing)
│   ├── backend/
│   │   ├── backend-mental-model.md        # S3 storage strategy, data transformation patterns
│   │   ├── testing.md                     # DDD layer test patterns
│   │   └── observability.md               # Python logging conventions
│   ├── frontend/
│   │   ├── testing.md                     # RTL patterns, error state testing
│   │   └── observability.md               # Console logging guidelines
│   ├── parser/
│   │   ├── parser-mental-model.md         # Demo file encoding, timeline offset, frame reconciliation
│   │   └── observability.md               # Tracing conventions
│   ├── error-handling.md                  # Cross-service error philosophy
│   └── observability.md                   # Cross-service logging standards
├── docs/                                  # Public documentation only
├── backend/CLAUDE.md                      # FastAPI conventions, ≤100 lines
├── frontend/CLAUDE.md                     # React/TS conventions, ≤100 lines
├── parser/CLAUDE.md                       # Rust/Axum conventions, ≤100 lines
│
└── private/                               # PRIVATE SUBMODULE (dashjump-private repo)
    ├── product/                           # Product Kata artifacts (Layer 1 + 2)
    │   ├── strategy/
    │   │   ├── vision.md                  # Vision, strategic intent, product initiative
    │   │   └── current-options.md         # Active bets/options
    │   └── experiments/
    │       ├── 001-experiment-name/
    │       │   ├── kata.md                # Full Product Kata structure
    │       │   └── learnings.md           # Post-experiment insights
    │       └── 002-experiment-name/
    │           ├── kata.md
    │           └── learnings.md
    ├── specs/                             # Feature specifications (Layer 3)
    │   └── NNN-feature-name.md            # Spec with task shards
    ├── plans/                             # Claude plan files + methodology docs
    ├── learnings.md                       # Cross-project discoveries and patterns
    ├── learnings-index.md                 # Lightweight router — check BEFORE loading learnings
    └── CONTEXT.md                         # Machine-switching state file
```

**Submodule setup:**
```bash
# One-time setup (create the private repo on GitHub first)
cd dashJump.gg
git submodule add git@github.com:YOUR_USERNAME/dashjump-private.git private
git commit -m "Add private submodule for strategy and specs"
git push
```

**Note on file paths:** Throughout this document and in all agent/command definitions, files in the private submodule are referenced as `private/...` (e.g., `private/product/strategy/vision.md`, `private/specs/NNN-feature.md`). This is the path subagents use to read them. Internally within the private repo, the paths are relative (e.g., `product/strategy/vision.md`).

### Machine-Switching Ritual

**Before closing (30 seconds):**
```
> /switch-machine
```

This slash command tells Claude: "Write our current progress, active experiment step, next actions, and any open questions to `private/CONTEXT.md`. Commit and push both repos."

**On the other machine (30 seconds):**
```
> git pull && git submodule update --remote
> Read private/CONTEXT.md and continue where we left off.
```

### What Syncs via Git (and what doesn't)

| Syncs via PUBLIC repo | Syncs via PRIVATE submodule | Does NOT sync |
|---|---|---|
| `CLAUDE.md` files (root + subdirectories) | `private/product/` (strategy, experiments) | `~/.claude/projects/` (auto memory) |
| `.claude/settings.json` | `private/specs/` (feature specs) | `~/.claude/session-memory/` |
| `.claude/commands/`, `agents/`, `skills/` | `private/learnings.md`, `learnings-index.md` | `CLAUDE.local.md` (gitignored) |
| `.claude/knowledge-management.md` | `private/plans/` (Claude plans + methodology) | Claude's conversation history |
| `.claude/rules/` (standards + mental models) | `private/CONTEXT.md` (machine-switch state) | |
| All source code (backend, frontend, parser) | | |

### Context Management Rules

- **Root CLAUDE.md:** ≤200 lines (~2,000 tokens). Tell Claude how to find information, don't embed it all.
- **Subdirectory CLAUDE.md:** ≤100 lines each. Lazy-loaded only when Claude reads files in that subtree.
- **Skills:** Descriptions load at start, full content loads on invocation. Keep brand skill under 5,000 tokens.
- **Spec task shards:** Each shard ≤2,000 tokens. Contains only what that specific unit of work needs.
- **Experiment files:** Keep `kata.md` focused. Move completed steps to `learnings.md` to prevent bloat.
- **MCP servers:** Maximum 3 active at once. Each adds tool definitions to system prompt.
- **Clear at 30%:** Don't wait for context to fill. Quality degrades noticeably past 30%.

---

## Part 5: Slash Commands

These make the workflow frictionless to use in Claude Code sessions.

### `/kata-check`

**`.claude/commands/kata-check.md`**
```markdown
Read the active experiment in private/product/experiments/ (find the one with
Status: active-experiment or discovery) and answer the five Coaching Kata
questions:

1. What is the target condition?
2. What is the actual condition now?
3. What obstacles do you think are preventing you from reaching the target
   condition? Which ONE are you addressing now?
4. What is your next step? What do you expect?
5. When can we go and see what we have learned from taking that step?

Then assess:
- Are we still within our one-week time-box?
- Is the current step still the right next action?
- Should we update the experiment file based on what we've learned?

If updates are needed, propose specific edits to the kata.md file.
```

### `/new-experiment`

**`.claude/commands/new-experiment.md`**
```markdown
Create a new experiment directory in private/product/experiments/.

1. Find the highest-numbered existing experiment directory
2. Create the next number (e.g., if 003 exists, create 004)
3. Ask me for:
   - Experiment name (short, descriptive)
   - Which option from current-options.md this tests (or "new option")
   - Target condition (must be specific and measurable)
   - Current condition (quantified baseline)
   - First obstacle to address
   - First experiment step (action + expected outcome)
   - Experiment type (concierge | concept-test | wizard-of-oz | build-to-learn)
   - Cost of Delay assessment (urgency + value)
4. Scaffold kata.md with the full Product Kata template
5. Create empty learnings.md
6. Update current-options.md if this is a new option

Remind me: "Steps should take no more than one week. What's the smallest
thing we can do to learn whether this hypothesis is true?"
```

### `/quick-fix`

**`.claude/commands/quick-fix.md`**
```markdown
Skip the full experiment/spec workflow for small changes (bug fixes, tweaks,
minor improvements). This is our adaptation of BMAD's Quick Flow and
AWS AI-DLC's adaptive depth.

Use this when:
- Bug fix with clear reproduction
- Style/formatting tweak
- Documentation update
- Dependency update
- Small refactor (< 50 lines changed)

Do NOT use this when:
- Building a new user-facing feature (needs experiment + spec)
- Making architectural changes (needs spec at minimum)
- Any change that affects coach-facing functionality (needs experiment validation)

Process:
1. Describe the change needed
2. Identify affected files
3. Implement directly
4. Run relevant tests
5. Self-review for convention compliance
```

### `/switch-machine`

**`.claude/commands/switch-machine.md`**
```markdown
Write the current working state to private/CONTEXT.md for machine-switching.

Include:
- Active experiment and current step
- What was just completed
- What's in progress (any uncommitted work?)
- Next planned action
- Any open questions or decisions needed
- Files recently modified
- Any context that would be lost (conversation insights, debugging state)

Format as a brief, scannable document that a fresh Claude session can read
and immediately resume from. Keep under 2,000 tokens.

After writing, remind me to commit and push BOTH repos:
  cd private && git add . && git commit -m "context: [brief description]" && git push && cd ..
  git add . && git commit -m "context: [brief description]" && git push
```

### `/toc`

**`.claude/commands/toc.md`**
```markdown
Show the section structure of a file with line numbers, then read only the
section you need. Useful for large files where you don't want to load
everything into context.

Usage: /toc [filepath]

Steps:
1. Run: grep -n "^## \|^### \|^# " $ARGUMENTS | head -40
2. Review the headers and line numbers
3. Use Read with a line range to load only the relevant section
   Example: Read private/specs/003-wave-priority.md lines 45-78

This avoids loading an entire spec or plan file when you only need one section.
```

### `/test-audit`

**`.claude/commands/test-audit.md`**
```markdown
Run a comprehensive test suite audit using the test-auditor subagent.

Scan all test files across all three services and report:
1. Coverage gaps (components/endpoints/functions with no tests)
2. Missing error path tests
3. Stale or skipped tests
4. Pattern compliance issues
5. Prioritized action list

Current known state:
- Frontend: 53 tests across 5 files (Vitest + RTL + Playwright browser)
- Backend: 37 tests across 10 files (pytest, DDD-layered)
- Parser: 0 tests (critical gap)

Use the test-auditor agent for this work. Output should be actionable —
each gap should specify what to test and estimated effort.

Recommended cadence: monthly, or before any major release/demo.
```

### `/consolidate-learnings`

**`.claude/commands/consolidate-learnings.md`**
```markdown
Consolidate draft learnings and revise project documentation files. Use the spec-writer
agent. This command combines draft promotion with session-end file revision.

## Part 1: Promote Drafts

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

## Part 2: Revise Related Files

For each promoted learning, check if it should propagate to other files:
- Should it become a permanent rule in .claude/rules/{service}/? If so, add it and
  mark the learning as "Graduated to: [path]"
- Does it affect an agent's "Before Starting Work" checklist?
- Does it invalidate anything in an existing spec or mental model?

For files touched during the current session:
- Check if any session insights should be captured but weren't
- Verify file paths still resolve correctly after any changes

Only write to spec-writer-owned files (rules, learnings, knowledge-management.md).
For Steven-owned files (agents, commands, skills, CLAUDE.md), flag findings but do not edit.

## Part 3: Report

After consolidation, report:
- Learnings promoted (with anchors)
- Drafts discarded (with reason)
- Files updated beyond learnings (rules, index, etc.)
- Current learnings.md token estimate
- Steven-owned files flagged for review (if any) — suggest running the
  dashjump-context-audit skill for full audit details

Recommended cadence: weekly (pairs with /kata-check), end-of-session, or when
## Drafts has 3+ entries.
```

---

## Part 6: Root CLAUDE.md Integration

Add these lines to the root `CLAUDE.md` to wire everything together. This is a pointer-based approach — tell Claude where to find things, don't embed everything.

```markdown
## Workflow

This project uses a Product Kata-driven development workflow.

### Key Locations
- Product strategy: private/product/strategy/
- Active experiments: private/product/experiments/ (find Status: active-experiment)
- Feature specs: private/specs/
- Machine-switch state: private/CONTEXT.md (read by Steven at session start only, NOT by subagents)

### Knowledge Management
- Before starting work, check private/learnings-index.md for relevant cross-project learnings
- Full knowledge management rules: .claude/knowledge-management.md
- Service mental models: .claude/rules/[service]/[service]-mental-model.md
- If you discover a cross-project pattern, append to private/learnings.md ## Drafts section
- Run /consolidate-learnings weekly to promote drafts and revise related files (spec-writer agent)

### Shared File Ownership
- Strategy files (vision.md, current-options.md): spec-writer only
- private/learnings-index.md: spec-writer only (updated during /consolidate-learnings)
- private/learnings.md (promoted entries): spec-writer only
- private/learnings.md ## Drafts section: any service agent may append

### Definition of Done (applies to ALL work)
Every completed unit of work must meet these standards before it's considered done:
- Code reviewed (use code-reviewer agent or self-review for quick-fixes)
- Tests written and passing for new/changed code
- Observability: logging instrumented per service conventions
- Security: no violations of dashjump-compliance skill checklist
- Conventions: follows relevant service CLAUDE.md patterns

Note: This is separate from feature-specific Success Criteria defined in each spec.
Success Criteria = "when is THIS feature done?" DoD = "what standards must ALL work meet?"

### Development Principles
- NEVER build without a linked experiment defining the outcome we're targeting
- Specs require task shards — atomic units a subagent can execute independently
- Each experiment step must be ≤ 1 week
- Use /kata-check weekly to review experiment progress
- Use /quick-fix for bugs and small changes (skip experiment/spec ceremony)
- Steven verifies at each shard boundary before next shard begins

### Context Budgets (enforce when creating/updating files)
- Root CLAUDE.md: ≤200 lines (~2,000 tokens)
- Subdirectory CLAUDE.md: ≤100 lines each
- Spec task shards: ≤2,000 tokens per shard
- Skills: keep under 5,000 tokens each
- Experiment kata.md: move completed steps to learnings.md to prevent bloat
- MCP servers: maximum 3 active simultaneously
- Clear at 30%: don't wait for context to fill — quality degrades past 30%
- Full budget reference: .claude/skills/dashjump-context-audit/references/context-budgets.md

### Project Health Auditing
- Run the dashjump-context-audit skill periodically to audit all .claude/ files
- Ownership rules: .claude/skills/dashjump-context-audit/references/ownership-map.md
- Quality criteria per file type: .claude/skills/dashjump-context-audit/references/

### Installed Plugins
These plugins enhance agent capabilities — agents should leverage them when relevant:
- LSP: typescript-lsp, pyright-lsp, rust-analyzer-lsp (real-time type checking per service)
- Testing: playwright (E2E browser testing)
- Quality: code-review, security-guidance, pr-review-toolkit, code-simplifier
- Design: frontend-design (production-grade UI)
- Workflow: claude-md-management, commit-commands, context7, explanatory-output-style

### Before Starting Any Feature Work
1. Check private/product/experiments/ for the active experiment
2. Read the current experiment's kata.md — what step are we on?
3. If building: find the spec in private/specs/ with task shards
4. Work from a single task shard — don't load the full spec into context
5. After completing a shard: run the "Verify before proceeding" check
```

---

## Part 7: Implementation Sequence

Execute these steps in order. Each step is independent and completable in one session.

### Step 1: Create Directory Structure and Private Submodule
1. Create the `dashjump-private` repo on GitHub (private).
2. Add it as a submodule: `git submodule add git@github.com:YOUR_USERNAME/dashjump-private.git private`
3. Create public repo directories and placeholder files from the Part 4 directory structure.
4. Create private repo directories: `private/product/strategy/`, `private/product/experiments/`, `private/specs/`.
5. Initialize `private/product/strategy/vision.md` and `private/product/strategy/current-options.md` with the templates from Part 2.
6. Commit both repos.

### Step 2: Create Slash Commands
Create all seven slash command files from Part 5 in `.claude/commands/`.

### Step 3: Create Subagent Definitions
Create all eight subagent files from Part 3 in `.claude/agents/`: `dashjump-designer`, `backend-python`, `frontend-react`, `rust-parser`, `code-reviewer`, `spec-writer`, `e2e-playwright`, and `test-auditor`. Do NOT create `infra-specialist` — that's in the deferred plan.

### Step 4: Create Skills
Create `.claude/skills/dashjump-brand/SKILL.md` with the template from Part 3, Phase 1. Mark color/typography sections as `[TO BE DEFINED — run brand discovery experiment first]`.

Create `.claude/skills/dashjump-compliance/SKILL.md` with the full security and compliance checklist from Part 3. This one is ready to use immediately — no discovery needed.

Create `.claude/skills/dashjump-context-audit/` with SKILL.md and the `references/` directory containing the seven quality criteria files, context-budgets.md, and ownership-map.md. This skill audits all .claude/ and private/ documentation files against type-specific rubrics.

### Step 5: Set Up Knowledge Management
Place the knowledge management rules in the public repo and the learnings in the private submodule:
- `.claude/knowledge-management.md` — rules for the 5-tier knowledge system (public — methodology, not proprietary)
- `private/learnings.md` — cross-project discoveries (private — contains coach feedback and strategic insights)
- `private/learnings-index.md` — lightweight router for finding relevant learnings (private — reveals what you've learned)

These files already exist and contain active learnings (demo timeline offset, S3 storage pattern, wave priority validation). Ensure they are committed to the correct repos.

### Step 6: Configure Settings
Create/update `.claude/settings.json`:
```json
{
  "plansDirectory": "./private/plans"
}
```

### Step 7: Update Root CLAUDE.md
Add the workflow section from Part 6 to the existing root `CLAUDE.md`. This includes the new Knowledge Management pointer section.

### Step 8: Create First Experiment
Use the `/new-experiment` command to scaffold the first experiment. Suggested: a brand discovery experiment to populate the design skill, since that unblocks all subsequent design work.

### Step 9: Install MCP Servers (Optional)
```bash
# Tailwind CSS tools for design subagent (when ready for design work)
# claude mcp add tailwindcss -s user -- npx tailwindcss-mcp-server

# Terraform — DEFERRED until infrastructure plan activates
# See private/plans/infra-subagent-plan-deferred.md
```

Note: MCP servers are optional at launch. The workflow functions without them. Add the Tailwind server when beginning brand discovery work. Limit to 3 active MCP servers maximum to manage context consumption.

---

## Part 8: How This Workflow Runs Day-to-Day

### Starting a Session
```
> git pull && git submodule update --remote
> Read private/CONTEXT.md    [if switching machines]
> /kata-check                [weekly or at start of new work]
```

### Working on an Experiment Step
1. Read the active experiment's `kata.md` — what's the current step?
2. If the step requires building: check for a spec in `private/specs/`
3. Work from a single task shard, not the full spec
4. Use the appropriate subagent for the domain (backend-python, frontend-react, etc.)
5. When step completes: update `kata.md` with what was learned
6. Run `/kata-check` to decide next action

### Completing an Experiment
1. Fill in all "Learned" fields in `kata.md`
2. Write `learnings.md` with expected vs actual, insights, strategy impact
3. Update `Status:` to `achieved` or `abandoned`
4. Update `private/product/strategy/current-options.md` with outcome
5. Ask: "Did we achieve the target condition? If not, do we iterate or move on?"
6. **Critical:** Do NOT start the next experiment until current achieves target condition or you've explicitly decided to abandon with documented reasoning

### Quick Fixes
```
> /quick-fix
> [Describe the bug/tweak]
> [Claude implements directly, runs tests, done]
```

### Ending a Session
```
> /switch-machine          [if you'll continue on another machine]
> cd private && git add . && git commit -m "..." && git push && cd ..
> git add . && git commit -m "..." && git push
```

### Weekly Cadence (from Product Kata)
- **Start of week:** `/kata-check` on active experiment
- **During week:** Work experiment steps, each ≤ 1 week
- **End of week:** Review what was learned, decide next step
- **End of week:** `/consolidate-learnings` — promote drafts, revise related files
- **Bi-weekly:** Check progress against target condition
- **Monthly:** Assess if strategic intent is still valid; run dashjump-context-audit skill

---

## Metrics: What We Track

Following Perri's framework, measure outcomes not outputs.

### What We DON'T Track
- Features shipped
- Lines of code
- Story points
- Velocity

### What We DO Track (per experiment)
- **Target condition progress:** Are we moving toward the measurable goal?
- **Learning velocity:** How quickly are experiments producing actionable insights?
- **Hypothesis accuracy:** Expected vs actual results (calibration over time)
- **Coach engagement:** Are partners actively using what we build to make decisions?
- **Time-to-validation:** How long from hypothesis to validated/invalidated?

### Leading Indicators (real-time signals)
- Coach responds to check-in within 24 hours (engagement)
- Coach asks unprompted question about analytics (activation)
- Coach references dashJump data in their coaching (task success)

### Lagging Indicators (validate over time)
- Coach retention week-over-week
- Willingness to pay (the ultimate validation metric)
- Referral to other coaches

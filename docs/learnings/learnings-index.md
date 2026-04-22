# Learnings Index

Quick reference to find relevant learnings by topic, service, and problem type. Use this index to decide if you need to load a learning from `private/learnings.md`.

See [`.claude/knowledge-management.md`](../.claude/knowledge-management.md) for full knowledge management system rules, learnings entry format, and anti-patterns.

## By Service

### Parser
- **Deadlock Entity Field Enums Not in Protobufs** — Use SendTables tooling, not proto files, to look up entity field types and enum values
  - When: Debugging unexpected entity field values, adding a new entity tracker, searching for enum definitions
  - See: private/learnings.md#deadlock-entity-field-enums-are-not-in-protobufs
  - Also see: `.claude/rules/parser/parser-mental-model.md` — "Entity Field Lookup Tools" section
- **Cargo Container-Only / Worktrees Not Auto-Mounted** — Running `cargo` commands from a git worktree requires a one-off Docker container
  - When: Starting Rust work from a worktree, `cargo: command not found` errors, setting up any worktree with Rust tasks
  - See: private/learnings.md#cargo-is-container-only-worktrees-are-not-auto-mounted

### Backend
- **S3 Storage vs JSONB** — Storage strategy for large per-match data
  - When: Making storage decisions, evaluating performance of large JSON objects
  - See: private/learnings.md#s3-storage-solves-jsonb-bottleneck-better-than-differential-encoding
  - Also see: `.claude/rules/backend/backend-mental-model.md`
- **parsedmatch Cache Invalidation on Parser Schema Changes** — Every parser output schema change requires a cache invalidation
  - When: Adding new parser output fields, changing field types, debugging "new field always zero/None", writing Alembic migrations touching parsedmatch
  - See: private/learnings.md#parsedmatch-cache-must-be-invalidated-on-parser-schema-changes

### Parser + Backend (cross-service)
- **Boss Objective Health: Parser Must Emit health=0 at Entity Delete** -- parser responsibility for correct carry-forward
  - When: Implementing entity trackers with health timelines, debugging stale liveness, reviewing carry-forward logic
  - See: private/learnings.md#boss-objective-health-parser-must-emit-health0-at-entity-delete

### Frontend
- (No frontend-specific cross-project learnings yet)

---

## By Problem Type

### Storage & Performance
- **S3 Storage vs JSONB** — Performance limits of JSONB for large objects
  - Affects: Backend storage strategy, Parser output format, Frontend data transfer
  - See: private/learnings.md#s3-storage-solves-jsonb-bottleneck-better-than-differential-encoding
  - When: Evaluating storage strategies, sizing infrastructure, optimizing performance

### Data Transformation
- **parsedmatch Cache Invalidation on Parser Schema Changes** — bump schema_version or delete rows on every parser output change
  - Affects: Backend (cache), Parser (output schema), development workflow
  - See: private/learnings.md#parsedmatch-cache-must-be-invalidated-on-parser-schema-changes
  - When: Parser schema changes, stale data bugs, Alembic migration authoring

### Debugging: Entity & Protocol Introspection
- **Deadlock Entity Field Enums Not in Protobufs** — SendTables tooling lookup path
  - Affects: Parser (primary), any service that needs to interpret raw entity field values
  - See: private/learnings.md#deadlock-entity-field-enums-are-not-in-protobufs
  - When: Debugging raw integer field values on Deadlock entities, finding enum definitions

### Objective Lifecycle
- **Boss Objective Health: Parser Must Emit health=0 at Entity Delete** -- carry-forward correctness across parser + backend
  - Affects: Parser (boss_tracker.rs), Backend (lane_pressure_service.py), any future objective tracker
  - See: private/learnings.md#boss-objective-health-parser-must-emit-health0-at-entity-delete
  - When: Implementing entity trackers with health timelines, debugging stale objective liveness, reviewing carry-forward logic

### Worktree / Infrastructure
- **Cargo Container-Only / Worktrees Not Auto-Mounted** -- Rust toolchain only in Docker; worktrees need a one-off container
  - Affects: Any agent doing parser Rust work from a non-main worktree
  - See: private/learnings.md#cargo-is-container-only-worktrees-are-not-auto-mounted
  - When: Starting parser work from a worktree, `cargo: command not found` errors

### Coach Feedback & Validation
- **Wave Priority > Kill Data** — Coach priorities for analytics features
  - Affects: Feature prioritization, roadmap decisions, what data to surface
  - See: private/learnings.md#wave-priority-tracking--raw-kill-data-coach-feedback-pattern
  - When: Prioritizing features, explaining roadmap decisions, validating new feature ideas

---

## Maintenance

### When to Add to This Index
- A new learning is added to `private/learnings.md`
- An existing learning becomes relevant to new service or problem type
- A new pattern is identified (appears 2+ times)

### Quarterly Review
- Check all links still work
- Consolidate duplicate entries
- Archive deprecated learnings
- Anchors are auto-generated from headings — if a heading in learnings.md changes, update all index references here

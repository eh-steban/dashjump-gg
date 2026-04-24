# About docs/

Hello! If you're here, you're probably curious about how my multi-agent workflow is setup.

This directory is a curated public snapshot of the engineering knowledge base that drives development on [dashJump.gg](https://dashjump.gg) -- a replay-analytics platform for Valve's Deadlock. The project is a three-service monorepo I've built using a multi-agent AI-assisted workflow (Python/FastAPI backend, React/TypeScript frontend, Rust/Axum data parser). 

## How this fits the broader workflow

The full workflow involves a mix of reverse engineering and product-driven development. Every feature is linked to a product experiment with a measurable outcome. Work flows through `spike -> discovery -> implementation`, with each step producing a planning artifact in `plans/`. Lessons from any step are captured in `learnings/` and fed back into future work. These plans have become instrumental when turning reverse engineering findings into executable code.

AI subagents (backend, frontend, parser, spec-writer, and others) read these documents as shared context. The reference docs and contracts in `references/` are the primary source of truth for cross-service work. They are designed to be precise and machine-readable, not just human-friendly prose. The engineering probes in `engineering/probes/` are how we discover what data Deadlock replays actually expose before committing to a feature design.

The result is a tight loop: probe the replay data, capture findings in references, plan in small shards, implement, record what was surprising in learnings. Each artifact in this directory is a byproduct of that loop.

---

## What's here

### `references/`

Low-level technical ground truth for the Rust parser service. These are the docs an engineer reaches for when wiring up a new Deadlock replay feature.

| File | What it covers |
|------|----------------|
| `citadel-messages-reference.md` | Full catalog of Citadel protobuf user-message IDs (300-366), fields, and observed firing rates across real replays |
| `citadel-gcmessages-common-reference.md` | Field-level breakdown of `CMsgMatchMetaDataContents` -- the post-match blob embedded in message ID 316 |
| `citadel-messages-supplemental.md` | Additional field-level notes from probe runs, cross-referenced to the main catalog |
| `entity-types-reference.md` | Known Deadlock entity class names (e.g. `CNPC_MidBoss`, `CCitadelPlayerPawn`) with descriptions |
| `entity-types-runtime-census.md` | Entity class census tallied from live replays via `probe_all_entity_classes` |
| `entity-fields-reference.md` | Entity property names and types extracted from SendTables, the authoritative field source for Deadlock |
| `entity-fields-supplemental.md` | Probe-validated field notes beyond what SendTables exposes directly |
| `deadlock-api-haste-reference.md` | API surface of the `deadlock-api/haste` Rust crate used for replay decoding |
| `contracts/backend-api.md` | Backend HTTP response contract (`GET /match/analysis/{match_id}`) -- source of truth for frontend domain types |
| `contracts/parser-output.md` | Parser output JSON schema -- source of truth for backend deserialization. Long-form field explanations live in `contracts/references.md` |

### `plans/`

Planning artifacts from the product-kata workflow. Subdirectories:

- **`spikes/`** -- Single narrow questions with a ≤1-day timebox. Produces a yes/no answer with evidence.
- **`discovery/`** -- Multi-unknown investigations where enough unknowns exist to block a design decision.
- **`implementation/`** -- Execution plans with atomic task shards, once unknowns are resolved. Active plans are at the top level; shipped plans move to `completed/` and abandoned ones to `archived/`.
- **`fixes/`** -- Narrow defect plans for bugs with a known root cause.

### `process/`

Plan templates for each plan type (spike, discovery, implementation, fix). Templates encode required sections, scope checks, and acceptance criteria conventions used by both humans and AI subagents.

### `learnings/`

Cross-project lessons captured after significant debugging sessions or architectural decisions. `learnings-index.md` is a quick-reference index organized by service and problem type; `learnings.md` contains the full entries.

### `engineering/`

Supporting engineering artifacts:

- **`probes/`** -- Standalone Rust (and occasionally Python) programs used to reverse-engineer Deadlock replay data. Each probe answers a specific question about what's observable in a replay (e.g. which entity classes exist, what fields fire on mid-boss kill). Probes are copy-paste-run tools -- not part of the production parser.
- **`observability-roadmap.md`** -- Planned logging, metrics, and tracing infrastructure across all three services.

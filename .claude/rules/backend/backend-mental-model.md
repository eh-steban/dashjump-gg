# Backend Mental Model

## S3 Storage Strategy

**Status:** Evaluated (not yet implemented)

PostgreSQL JSONB storage fails at scale for match data. Each parsed match produces 15-18 MB of JSON. At this size, JSONB hits hard limits: slow queries, large row sizes, and expensive full-object updates.

**Chosen architecture:**
- Store raw + transformed match JSON in S3 (one object per match)
- Keep lightweight metadata in PostgreSQL (`match_id`, `s3_key`, `duration_seconds`, `status`)
- API layer fetches S3 object on cache miss; PostgreSQL answers metadata queries

**Why not differential encoding?**
Differential encoding (storing deltas between frames) reduces storage size but adds read-time reconstruction complexity with no query performance benefit. S3 achieves storage efficiency without the reconstruction overhead.

**Key constraint:** JSONB reserved for small metadata only. Any new large-data feature defaults to S3 storage pattern.

**See:** `private/learnings.md` — "S3 Storage Solves JSONB Bottleneck" for the cross-project summary.

---

## DDD Layer Gotchas

(Populate as patterns emerge from code review)

## Data Transformation Pipeline

(Populate after S3 migration is implemented — the mapper layer will have non-trivial logic)

---

**See `.claude/knowledge-management.md` for when and how to populate this file.**

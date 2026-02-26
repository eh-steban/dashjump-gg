# Backend Mental Model

## Status: TODO

This file is a stub. Populate it as architectural patterns and constraints are discovered in the backend service.

## What Goes Here

Service-specific architecture deep-dives that are:
- Non-obvious constraints that would cause expensive debugging if not documented
- Data transformation patterns unique to this service
- Patterns emerging from code review that recur across multiple PRs
- Interaction between DDD layers that isn't obvious from the code

## Candidate Topics (to be expanded)

### S3 Storage Strategy
The backend is evaluating S3/Parquet as a replacement for JSONB storage for large match data (~15-18 MB per match). See `private/learnings.md` — "S3 Storage Solves JSONB Bottleneck" for the cross-project summary.

When this is implemented, document here:
- How match data is partitioned in S3
- How PostgreSQL metadata relates to S3 objects
- Cache key strategy
- How the API layer retrieves and assembles responses

### DDD Layer Gotchas
(Populate as patterns emerge)

### Data Transformation Pipeline
(Populate after S3 migration is implemented — the mapper layer will have non-trivial logic)

---

**See `.claude/knowledge-management.md` for when and how to populate this file.**

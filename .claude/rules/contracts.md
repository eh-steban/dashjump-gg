---
paths:
  - "parser/src/domain/**"
  - "backend/app/domain/**"
  - "frontend/src/domain/**"
---
# Interservice Contract Standards

Rules for all agents working on features that cross service boundaries.

## What Is a Contract

A contract is the agreed JSON shape at a service boundary:

| Contract | Boundary | Spec file |
|----------|----------|-----------|
| Parser output | parser -> backend | `private/specs/contracts/parser-output.md` |
| Backend API | backend -> frontend | `private/specs/contracts/backend-api.md` |

These files are the single source of truth. When code and spec diverge, update the spec first,
then update the consuming service.

## Ownership

| Contract | Owner (updates spec) | Consumer (reads spec) |
|----------|---------------------|----------------------|
| `parser-output.md` | `rust-parser` agent | `backend-python` agent |
| `backend-api.md` | `backend-python` agent | `frontend-react` agent |

## Contract-First Rule

**A task shard that adds, removes, or renames a field crossing a service boundary must update
the contract spec before touching implementation code.**

This applies whenever:
- A Rust serde struct in `parser/src/domain/` changes serialized field names or types
- A Python Pydantic model in `backend/app/domain/` changes fields on an API response type
- A TypeScript interface in `frontend/src/domain/` is updated to match backend changes

## Enforcement Checkpoints

When completing a shard that touches a contract:

**Parser agent:**
- [ ] Does `ParsedMatchResponse` in backend still compile against `parser-output.md`?
- [ ] Are all new fields documented in `parser-output.md` with type, required/optional, and notes?

**Backend agent:**
- [ ] Does `TransformedMatchData` still match `backend-api.md`?
- [ ] Does `ParsedMatchResponse` still match `parser-output.md`?
- [ ] Are `test_match_api.py` schema tests still passing?

**Frontend agent:**
- [ ] Do TypeScript interfaces in `frontend/src/domain/` match `backend-api.md`?
- [ ] No new fields inferred from Python files -- read `backend-api.md` instead.

## Phase 0 in Implementation Plans

Any implementation plan spanning multiple services must include a Phase 0 that locks the
contract before parallel work begins. See `private/templates/plans/implementation.md`.

## When Contracts Are Not Required

Contracts apply only to JSON at service boundaries. Internal refactors within a single service
(renaming a private struct, splitting a helper function) do not require spec updates.

## Versioning

The backend uses `schema_version` (currently 1) to gate cached data. Increment schema version
when the `backend-api.md` contract has a breaking change (field removed or type narrowed).
Non-breaking additions (new optional field) do not require a version bump.

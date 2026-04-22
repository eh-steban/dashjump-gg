# Parser Dispatch Exhaustiveness Test Shape Spike

> **File location:** `private/plans/spikes/`

## Context

The mid-boss damage dispatch panic fix (`private/plans/fixes/midboss-damage-dispatch-panic.md`, 2026-04-14) added a short-circuit for `CNPC_MIDBOSS_ENTITY` in `get_damage_entity_id`. Two tests were planned: a point regression test asserting `get_damage_entity_id` returns `Ok` for a mid-boss entity, and an exhaustiveness guard iterating every `*_ENTITY` constant. Both require constructing a mock `haste::Entity`, which has no public constructor -- so neither test could be written without scope expansion.

---

## Question

What is the minimal seam that allows `get_damage_entity_id`'s hash-dispatch logic to be unit-tested without patching haste, adding a public `Entity` constructor, or refactoring the function signature?

---

## Assumptions

### To Validate

- [ ] `haste::Entity` will remain without a public constructor in the near term -- *How to check: read haste changelog / open issues on deadlock-api/haste; check if any `#[cfg(test)]` or `pub(crate)` constructors exist in the current checkout.*
- [ ] Extracting the dispatch logic into a free function `dispatch_entity_hash(hash: u64, is_boss: bool) -> DispatchKind` would allow full coverage without changing the public API -- *How to check: sketch the refactor in a branch and verify call sites still compile.*

### Accepted (not tested here)

- The fix itself is correct -- the short-circuit was code-reviewed and manual-checked against replay 55423930. Risk if wrong: a second bad dispatch arm would still require an integration test or real replay to catch.

---

## Agent & Timebox

**Agent:** rust-parser
**Timebox:** 1 day

---

## Research Standards

Follow `.claude/rules/research.md` for confidence labels, citation format, and scope discipline.

---

## Investigation Approach

1. Read `haste::Entity` struct in the container checkout (`/usr/local/cargo/git/checkouts/haste-*/`) -- confirm no `#[cfg(test)]` or `pub(crate) fn new` constructor exists.
2. Sketch a pure-function extraction: pull the `if hash == X { return Y }` ladder out of `get_damage_entity_id` into a `fn classify_damage_entity(hash: u64, is_boss: bool) -> EntityDamageClass` (or similar). This function takes only primitives and is trivially testable. Assess whether all current callers fit the new shape without churn.
3. Evaluate the alternative: a `#[cfg(test)]` builder on `MyVisitor`'s test module that constructs a thin wrapper instead of a real `Entity`. This avoids changing the dispatch function signature but couples tests to `MyVisitor` internals.
4. Pick the approach with the smallest diff that achieves full exhaustiveness coverage and document it as the recommended seam.

---

## Findings *(agent fills in)*

**Answer:** [Direct answer to the question above]

**Supporting evidence:**
- [citation: file:line or proto field or URL] -- [what was observed]

**Overall confidence:** `confirmed` / `inferred` / `hypothesis`

### Assumptions check

- [ ] `haste::Entity` has no public constructor -- held / invalidated -- [evidence]
- [ ] Free-function extraction enables full coverage -- held / invalidated -- [evidence]

---

## Learnings Output

- [ ] Draft entry appended to `private/learnings.md` ## Drafts
- [ ] Follow-up questions or spikes needed:

  None anticipated -- this spike should unblock implementation directly.

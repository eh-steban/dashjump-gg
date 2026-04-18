//! Tests for replay_parser module
//!
//! ## Testability gap -- dispatch path
//!
//! `get_damage_entity_id` takes `&Entity` (haste type), which has no public
//! constructor. There is no way to create a mock Entity in unit tests without
//! either patching haste or refactoring `get_damage_entity_id` to accept a
//! `u64` hash directly -- both are out of scope for the mid-boss dispatch fix.
//!
//! A point regression test for the `CNPC_MIDBOSS_ENTITY` short-circuit and an
//! exhaustiveness guard over all `*_ENTITY` constants in `entities/constants.rs`
//! were deferred to spike `private/plans/spikes/parser-dispatch-exhaustiveness-test-shape.md`,
//! which will design a testable seam that does not require API changes to haste.
//!
//! Until that spike lands, the fix is verified by:
//! - Manual check: parsing replay 55423930 via `GET /match/analysis/55423930` no
//!   longer panics at `parser/src/replay_parser.rs`.
//! - Code review: the short-circuit mirrors the existing `CPROJECTILE_PRIEST_SLIDETRAP_ENTITY`
//!   arm at line 296 and is trivially correct.

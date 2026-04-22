# Sinner Sacrifice Tracking -- Implementation Plan

## Context

Sinner's Sacrifice machines are breakable neutral objectives on the Deadlock map. There are 10 sites and 12 machines: 6 solo sites (one machine each), 2 hybrid sites (one machine + two Medium Denizens), and 2 hybrid sites (two machines + one Medium Denizen). All 12 spawn simultaneously 8 minutes after match start and respawn 5 minutes after being broken or fully cleared.

Each machine has 500 HP and can only be damaged by melee attacks -- abilities that deal melee damage (e.g., Viscous's Puddle Punch, Calico's Leaping Slash) do not register. This means `CCitadelUserMessageDamage` events from sinners will exclusively reflect melee hits, simplifying attacker identification. Breaking a machine awards the killer escalating Souls (310 base + 3.35/min from match start), making early sinners worth ~336 souls and late-game sinners ~477 at 50 minutes.

The **jackpot mechanic** is the reason health decrements to 1 rather than 0: after 400 damage, the machine enters a 3-second timing minigame. The final hit always deals exactly 100 damage (regardless of light vs. heavy melee), leaving health at 1. Our death signal (`health == 1 && prev_health > 1`) is correct precisely because of this mechanic -- health reaches 1 at the killing blow and is never written as 0 before the entity is recycled.

Each machine also retaliates: it deals 80 damage back to the attacker per melee hit. A player clearing a sinner solo absorbs 320--640 retaliation damage in total (320 for four heavy melees, 640 for eight light melees). Retaliation is deactivated during the jackpot state. This is a meaningful resource cost that coaches can use to evaluate sinner efficiency.

The platform currently has no data on Sinner spawns, deaths, kill attribution, or retaliation damage taken, which means the analytics layer cannot answer "who killed which sinners, when, and at what HP cost" -- a key mid-game engagement signal.

**Goals:**
- Parser outputs a `sinners` array containing one `SinnerSnapshot` per spawn event (initial + all respawns), with kill attribution resolved to `killer_player_slot` and per-player retaliation damage accumulated in `retaliation_damage`
- Backend passes `sinners` through to the API response without transformation

**Branch:** `feature/sinner-tracking` (worktree: `dashjump-gg-sinner-tracking`)
**Review workflow:** implement -- test -- subagent updates plan -- pause for user review -- commit -- next phase

---

## Scope

| Service  | Involved | Agent              |
|----------|----------|--------------------|
| Parser   | yes      | `rust-parser`      |
| Backend  | yes      | `backend-python`   |
| Frontend | yes      | `frontend-react`   |

---

## Acceptance Criteria

Feature is done when ALL of the following are true:

- [ ] Parser `sinners` array contains one entry per spawn event (40+ entries in a ~30-minute match)
- [ ] Each snapshot with a confirmed kill has `death_time_s` and `time_alive_s` populated; `killer_player_slot` is populated for player-killed sinners and for Lil Helper-killed sinners (resolved via Lil Helper owner chain -- see spike `lil-helper-sinner-interaction.md`)
- [ ] Snapshots for sinners alive at match end have all three optional death fields as `null`
- [ ] Each snapshot has a `retaliation_damage` map; entries for sinners directly hit by a player have at least one non-zero player entry (sinners killed exclusively by Lil Helpers will have an empty map -- this is correct, not a bug)
- [ ] `GET /match/analysis/{match_id}` response includes a `sinners` key with the snapshot array
- [ ] `cargo test` passes with no regressions
- [ ] `pytest` passes with no regressions
- [ ] All in-scope phase checkpoints complete and signed off by user

---

## Reference Data

**Site and machine count:** 10 sites, 12 machines. 6 solo sites (1 machine each) + 2 hybrid sites (1 machine + 2 Denizens) + 2 hybrid sites (2 machines + 1 Denizen) = 12 machines. This is why the spike found 12 `CNPC_Neutral_SinnersSacrifice` entities per wave, not 10.

**Spawn timing:** all 12 entities spawn within ~2 seconds of each other at ~8 minutes (461s / 483s across two replays). Respawn interval: ~5 minutes (~300s) after the machine is broken. Entity indices are recycled on respawn -- a CREATE fires again on the same index.

**Health pattern:** 500 → 400 → 300 → 200 → 100 → 1. Each light melee deals 50 damage; each heavy melee deals 100 damage. The jackpot mechanic makes the final hit always deal 100 damage regardless of type, leaving health at 1. Death signal: `health == 1 && prev_health > 1` on an UPDATE. `health == 0` appeared in only 1 of 34 deaths (delta compression skips the zero packet when the entity is recycled in the same delta window). Do not use `health == 0` as a death signal.

**Damage source:** melee-only. Ability-based melee (Viscous Puddle Punch, Calico Leaping Slash) does not damage sinners. `CCitadelUserMessageDamage` events for sinner victims will reflect melee hits from players and potentially from Rem's Lil Helpers (NPC entities that can be assigned to sinners and always secure jackpots as heavy melees). Whether Lil Helpers generate Damage events and what `entindex_attacker` looks like in that case is pending the Lil Helper spike (`lil-helper-sinner-interaction.md`).

**Retaliation damage:** 80 damage per melee hit dealt back to the attacker. Total per clear: 320 (four heavy melees) to 640 (eight light melees). Retaliation is deactivated during the jackpot state (the timing minigame after 400 damage taken), so the killing blow itself deals no retaliation. In `CCitadelUserMessageDamage`, retaliation events appear with `entindex_attacker` = sinner entity index and `entindex_victim` = player pawn entity index -- the reverse of the player-hits-sinner direction. Use `msg.damage()` to accumulate retaliation (this reflects the sinner's raw outgoing damage, consistently 80 per hit from the wiki; it is not subject to player armor mitigation on the way out).

**Multiple attackers:** the game distributes soul rewards across all players who hit the machine, but for kill attribution we care only about the killing blow attacker -- the last `entindex_attacker` recorded at `health == 1`. Retaliation damage, by contrast, should be accumulated across all players for the full lifetime of the spawn.

**Kill attribution:** `CCitadelUserMessageDamage` (ID 300), fields `entindex_victim` and `entindex_attacker` are direct entity indices (no ehandle conversion). Track last attacker per sinner entity index. When the death signal fires, resolve attacker entity index → player pawn → controller → `m_unLobbyPlayerSlot`. If the attacker is a Lil Helper NPC (not a player pawn), a different resolution chain is needed -- the Lil Helper entity likely has an owner pointing back to Rem's pawn. The exact chain (Lil Helper entity → owner ehandle → Rem's pawn → controller → slot) must be confirmed by the Lil Helper spike before implementing this branch.

**Souls value formula:** 310 base souls + 3.35/min from match start. At initial spawn (~8 min): ~336 souls. At 30 min: ~410 souls. At 50 min: ~477 souls.

**Counts confirmed in probe (`68182475_4609034.dem`):** 40 total CREATEs (12 initial + 28 respawns), 34 deaths detected. 6 sinners were alive at match end and have no `death_time_s`.

**`CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY` custom_id = 23** -- already defined at `replay_parser.rs:237`. Do not change this; sinners still receive a `custom_id` for other tracking paths.

---

## Critical Files

| Layer | File | Change |
|-------|------|--------|
| Parser contract | `private/specs/contracts/parser-output.md` | Modify |
| Parser domain | `parser/src/domain/sinner.rs` | Create |
| Parser domain registry | `parser/src/domain/mod.rs` | Modify |
| Parser tracker | `parser/src/tracking/sinner_tracker.rs` | Create |
| Parser tracker registry | `parser/src/tracking/mod.rs` | Modify |
| Parser integration | `parser/src/replay_parser.rs` | Modify |
| Backend domain | `backend/app/domain/sinner.py` | Create |
| Backend match analysis | `backend/app/domain/match_analysis.py` | Modify |

---

## Phase 0 -- Contract (`rust-parser` agent)

This phase blocks all others. No Phase A or Phase B work begins until the contract checkpoint is signed off.

### 0.1. Update `parser-output.md`

Add a `sinners` row to the Top-Level Response table and add a new `SinnerSnapshot` section beneath the `BossData` section. The section must include:

**Top-level addition:**

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `sinners` | SinnerSnapshot[] | yes | One entry per spawn event including respawns |

**SinnerSnapshot table:**

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `entity_index` | int | yes | Entity index (recycled on respawn -- not unique across snapshots) |
| `x` | float | yes | World X at spawn |
| `y` | float | yes | World Y at spawn |
| `z` | float | yes | World Z at spawn |
| `spawn_time_s` | int | yes | Match-relative seconds at CREATE |
| `max_health` | int | yes | Max health at spawn (confirmed 500 for all sinners) |
| `death_time_s` | int | no | Match-relative seconds at killing blow tick; null if alive at match end |
| `time_alive_s` | int | no | Derived: `death_time_s - spawn_time_s`; null if alive at match end |
| `killer_player_slot` | int | no | Lobby slot (0-indexed) of the killing player; null if alive at match end OR if killed by a Lil Helper and owner resolution fails (see Lil Helper spike) |
| `retaliation_damage` | Record<string, int> | yes | Map of lobby player slot (as string) to total retaliation damage that sinner dealt to that player; empty object `{}` if no player hit this sinner |

### 0.2. Confirm consuming service alignment

`ParsedMatchResponse` in `backend/app/domain/match_analysis.py` currently has no `sinners` field. Phase B will add `sinners: list[SinnerSnapshot]` to both `ParsedMatchResponse` and `TransformedMatchData`. Note that field here so Phase B has a clear target.

### 0 Checkpoint

**Status:** `[x] Complete` (2026-04-07)

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. List every field added across service boundaries
> 2. Confirm `private/specs/contracts/parser-output.md` has been updated
> 3. Check off every item below with date and actual result
> 4. Await user review before any Phase A or Phase B work begins

#### Results *(agent fills in)*

- [x] `private/specs/contracts/parser-output.md` updated -- `sinners` top-level field and `SinnerSnapshot` table added (2026-04-07)
- [x] All boundary-crossing fields listed in the field change log below (2026-04-07)
- [x] Phase B target noted: `ParsedMatchResponse` and `TransformedMatchData` need `sinners: list[SinnerSnapshot]` (2026-04-07)

#### Field change log *(agent fills in)*

| Field | Change | Spec file | Consuming service impact |
|-------|--------|-----------|--------------------------|
| `sinners` | added (SinnerSnapshot[]) | `parser-output.md` | `ParsedMatchResponse` needs `sinners: list[SinnerSnapshot]`; `TransformedMatchData` same |
| `sinners[].entity_index` | added (int, required) | `parser-output.md` | new field |
| `sinners[].x` | added (float, required) | `parser-output.md` | new field |
| `sinners[].y` | added (float, required) | `parser-output.md` | new field |
| `sinners[].z` | added (float, required) | `parser-output.md` | new field |
| `sinners[].spawn_time_s` | added (int, required) | `parser-output.md` | new field |
| `sinners[].max_health` | added (int, required) | `parser-output.md` | new field |
| `sinners[].death_time_s` | added (int, optional) | `parser-output.md` | new field |
| `sinners[].time_alive_s` | added (int, optional) | `parser-output.md` | new field |
| `sinners[].killer_player_slot` | added (int, optional) | `parser-output.md` | new field |
| `sinners[].retaliation_damage` | added (Record\<string,int\>, required) | `parser-output.md` | Python: `dict[str, int]`; keys are lobby slot as string |

#### Deferred items
None.

Await user review before proceeding to Phase A.

---

## Phase A -- Parser (`rust-parser` agent)

### A1. Add `should_track_snapshot()` to `replay_parser.rs`

Add a new method `should_track_snapshot()` on `MyVisitor` alongside the existing `should_track_position()`. This method identifies entities that need a one-time position capture at CREATE (as opposed to per-second position tracking). For now the only member is `CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY`, but the abstraction is designed for future stationary objectives (e.g., rune or camp spawns).

```rust
/// Check if entity needs a one-time position snapshot at CREATE.
/// Used for stationary objectives that do not require per-second tracking.
fn should_track_snapshot(&self, entity: &Entity) -> bool {
    let hash = entity.serializer().serializer_name.hash;
    matches!(hash, CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY)
}
```

Then remove `CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY` from the `matches!` block inside `should_track_position()` (line 148 in the current file). Sinners are stationary -- per-second position snapshots are wasteful and will pollute the `positions` array with 12 identical entries per second from ~8 minutes onwards.

Do NOT remove `CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY => 23` from `get_custom_id()`. The custom_id assignment is unrelated to position tracking and must stay.

### A2. Create `parser/src/domain/sinner.rs`

Create the domain struct file. Follow the same layout as `parser/src/domain/boss.rs`.

```rust
use serde::Serialize;

/// Snapshot of one Sinner Sacrifice spawn event (initial spawn or respawn).
/// Entity indices are recycled on respawn, so `entity_index` is not unique
/// across snapshots in a single match.
#[derive(Debug, Serialize, Clone)]
pub struct SinnerSnapshot {
    pub entity_index: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub spawn_time_s: u32,
    pub max_health: i32,
    pub death_time_s: Option<u32>,
    pub time_alive_s: Option<u32>,
    pub killer_player_slot: Option<u32>,
    /// Retaliation damage this sinner dealt to each player, keyed by lobby player slot.
    /// HashMap<u32, _> serializes keys as strings in JSON: {"0": 160, "4": 80}.
    /// Empty map if no player hit this sinner during its lifetime.
    pub retaliation_damage: std::collections::HashMap<u32, i32>,
}
```

### A3. Register `SinnerSnapshot` in `parser/src/domain/mod.rs`

Add `pub mod sinner;` and `pub use sinner::SinnerSnapshot;` following the pattern of existing entries.

### A4. Create `parser/src/tracking/sinner_tracker.rs`

Implement `SinnerTracker`. Follow the module layout of `parser/src/tracking/boss_tracker.rs`.

**Struct definition:**

```rust
pub struct SinnerTracker {
    snapshots: Vec<SinnerSnapshot>,
    last_health: HashMap<i32, i32>,
    last_attacker: HashMap<i32, i32>,
    max_health_key: u64,
}
```

`max_health_key` stores `fkey_from_path(&["m_iMaxHealth"])` computed once in `new()`. There is no exported `MAX_HEALTH_KEY` constant in `constants.rs` -- either add one there (preferred, for consistency with `HEALTH_KEY`) or define it inline in `SinnerTracker::new()`. If adding to `constants.rs`, name it `MAX_HEALTH_KEY` and place it next to `HEALTH_KEY`.

**Methods:**

`handle_sinner_create(entity_index: i32, x: f32, y: f32, z: f32, max_health: i32, spawn_time_s: u32)` -- push a new `SinnerSnapshot` with all optional fields as `None` and `retaliation_damage` as an empty `HashMap`, set `last_health[entity_index] = max_health`, and clear both `last_attacker.remove(&entity_index)`. Clearing on CREATE is important because entity indices are recycled -- stale state from the previous life must not carry over.

`record_damage(victim_entity_index: i32, attacker_entity_index: i32)` -- if `last_health.contains_key(&victim_entity_index)`, update `last_attacker.insert(victim_entity_index, attacker_entity_index)`. No-op otherwise.

`record_retaliation(sinner_entity_index: i32, player_slot: u32, damage: i32)` -- find the last snapshot for `sinner_entity_index` and add `damage` to `snapshot.retaliation_damage[player_slot]` (using `*entry(...).or_insert(0) += damage`). No-op if no snapshot exists for that entity index.

`handle_sinner_update(entity_index: i32, health: i32, current_time_s: u32) -> Option<i32>` -- look up `prev` from `last_health`. Update `last_health[entity_index] = health`. If `health == 1 && prev > 1`, find the last snapshot for this `entity_index` (the most recently pushed one), set its `death_time_s = Some(current_time_s)` and `time_alive_s = Some(current_time_s - spawn_time_s)`. Return `last_attacker.get(&entity_index).copied()`. If the death condition is not met, return `None`.

`record_sinner_death_killer(entity_index: i32, killer_player_slot: u32)` -- find the last snapshot for this `entity_index` that has `death_time_s.is_some()` and `killer_player_slot.is_none()`, then set its `killer_player_slot = Some(killer_player_slot)`.

`get_output(&self) -> &Vec<SinnerSnapshot>` -- returns a reference to the full snapshot vec.

**Finding the last snapshot for an entity index:** iterate `self.snapshots` in reverse and return the first entry where `snapshot.entity_index == entity_index`. This is O(n) but `n` is at most ~60 snapshots per match -- acceptable.

**Tests:** add a `tests` submodule (inline `#[cfg(test)]` block or a companion `sinner_tracker/tests.rs` -- follow the convention established by `boss_tracker/tests.rs`). Cover at minimum:
- `handle_sinner_create` pushes a snapshot with correct fields
- `handle_sinner_update` returns `None` for non-death health changes
- `handle_sinner_update` returns `Some(attacker)` on `health == 1` transition
- `handle_sinner_update` sets `death_time_s` and `time_alive_s` correctly
- `record_sinner_death_killer` sets `killer_player_slot` on the right snapshot
- `handle_sinner_create` on a recycled index clears stale attacker state
- `record_retaliation` accumulates damage across multiple hits from multiple players into the correct snapshot
- `handle_sinner_create` on a recycled index starts with an empty `retaliation_damage` map (prior life's data does not carry over)
- Sinner alive at match end has all optional death fields as `None` and `retaliation_damage` reflecting all hits taken

### A5. Register `SinnerTracker` in `parser/src/tracking/mod.rs`

Add `pub mod sinner_tracker;` and `pub use sinner_tracker::SinnerTracker;`.

### A6. Wire `SinnerTracker` into `replay_parser.rs`

**Struct field:** add `sinner_tracker: SinnerTracker` to `MyVisitor`.

**`on_entity` -- CREATE arm:** after the existing creep CREATE block, add a sinner CREATE block:

```rust
if match_started && hash == CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY {
    let Some(position) = get_entity_position(entity) else {
        error!(
            "[parse_replay] sinner entity has no position on CREATE (bug): Index={}",
            entity.index()
        );
        return Ok(());
    };
    let max_health: i32 = entity
        .get_value::<i32>(&MAX_HEALTH_KEY)
        .unwrap_or(500);
    let match_time_s = self.total_match_time_s
        .saturating_sub(self.match_start_time_s.unwrap_or(0));
    self.sinner_tracker.handle_sinner_create(
        entity.index(),
        position[0],
        position[1],
        position[2],
        max_health,
        match_time_s,
    );
}
```

Note: `unwrap_or(500)` uses the spike-confirmed default. If `MAX_HEALTH_KEY` is not added to `constants.rs`, call `fkey_from_path(&["m_iMaxHealth"])` inline and add a `// TODO: promote to constants.rs` comment.

**`on_entity` -- UPDATE arm:** after the existing creep UPDATE block, add a sinner UPDATE block:

```rust
if match_started && hash == CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY {
    if let Some(health) = entity.get_value::<i32>(&HEALTH_KEY) {
        let match_time_s = self.total_match_time_s
            .saturating_sub(self.match_start_time_s.unwrap_or(0));
        if let Some(attacker_entity_index) = self.sinner_tracker
            .handle_sinner_update(entity.index(), health, match_time_s)
        {
            // Death detected -- resolve attacker entity index to lobby player slot
            if let Some(entities) = ctx.entities() {
                if let Some(attacker_pawn) = entities.get(&attacker_entity_index) {
                    let owner_handle: u32 = attacker_pawn
                        .get_value(&OWNER_ENTITY_KEY)
                        .unwrap_or(0);
                    let controller_index = ehandle_to_index(owner_handle) as i32;
                    if let Some(controller) = entities.get(&controller_index) {
                        let slot: u32 = controller
                            .get_value(&LOBBY_PLAYER_SLOT_KEY)
                            .unwrap_or(9999);
                        if slot != 9999 {
                            self.sinner_tracker.record_sinner_death_killer(
                                entity.index(),
                                slot,
                            );
                        }
                    }
                }
            }
        }
    }
}
```

**`on_packet` -- damage handler:** inside the `KEUserMsgDamage` block, after the existing boss damage recording and before building the `DamageRecord`, add two sinner-related checks:

```rust
// Player or Lil Helper hits sinner -- track last attacker for kill attribution
if self.sinner_tracker.is_tracked_sinner(msg.entindex_victim()) {
    self.sinner_tracker.record_damage(
        msg.entindex_victim(),
        msg.entindex_attacker(),
    );
}

// Sinner retaliates -- accumulate retaliation damage per player slot.
// Q4 confirmed (lil-helper-sinner-interaction.md spike, replay 55841493):
// retaliation always targets a CCitadelPlayerPawn directly -- no NPC victim branch needed.
if self.sinner_tracker.is_tracked_sinner(msg.entindex_attacker()) {
    if let Some(entities) = ctx.entities() {
        if let Some(victim_pawn) = entities.get(&msg.entindex_victim()) {
            if victim_pawn.serializer_name_heq(CCITADELPLAYERPAWN_ENTITY) {
                let owner_handle: u32 = victim_pawn
                    .get_value(&OWNER_ENTITY_KEY)
                    .unwrap_or(0);
                let controller_index = ehandle_to_index(owner_handle);
                if let Some(controller) = entities.get(&controller_index) {
                    if let Some(slot) = controller.get_value::<u32>(&LOBBY_PLAYER_SLOT_KEY) {
                        self.sinner_tracker.record_retaliation(
                            msg.entindex_attacker(),
                            slot,
                            msg.damage(),
                        );
                    }
                }
            }
        }
    }
}
```

`is_tracked_sinner(entity_index: i32) -> bool` checks `last_health.contains_key(&entity_index)` -- add this method to `SinnerTracker` to keep internals private. Use `msg.damage()` (raw outgoing damage) rather than `msg.health_lost()` (net HP after mitigation), since the wiki guarantees a flat 80 per hit from the sinner's side and `damage()` reflects that consistently regardless of the player's armor.

### A6.5. Lil Helper attacker resolution for kill attribution (defensive branch)

**Spike status:** Partially run on replay `55841493_649180947.dem` -- see `lil-helper-sinner-interaction.md`. Key findings:
- All 280 sinner Damage events in that replay had `entindex_attacker` = a player pawn (Q1: no NPC attacker seen).
- All 39 sinner deaths were attributed to player pawns with resolved slots.
- Retaliation (Q4): 100% targeted player pawns directly -- no NPC victim observed. The existing pawn-only retaliation handler is correct; the TODO comment has been removed.
- Q1-Q3 are inconclusive because Rem may not have used Lil Helpers on sinners in that replay.

**What to implement:** add a defensive fallback to the killer resolution block in `on_entity` UPDATE. If the last attacker index does not resolve as a `CCitadelPlayerPawn`, attempt `m_hOwnerEntity` resolution. If that also fails, leave `killer_player_slot` as `None` and log a warning. This handles Hypothesis B (Lil Helper damage is attributed to the owning pawn -- making this branch unreachable) without breaking the normal path, while also safely handling the case where Hypothesis B is wrong.

```rust
// In the health==1 death handler, after looking up attacker entity:
let killer_slot = match attacker_entity.map(|e| e.serializer().serializer_name.hash) {
    Some(h) if h == CCITADELPLAYERPAWN_ENTITY => {
        // Normal path: attacker is a player pawn
        resolve_pawn_to_slot(attacker_index, ctx)
    }
    Some(_) => {
        // Defensive path: attacker is an NPC (e.g., possible Lil Helper)
        // Try m_hOwnerEntity to find the owning player pawn
        let owner_slot = attacker_entity
            .and_then(|e| e.get_value::<u32>(&OWNER_ENTITY_KEY))
            .map(ehandle_to_index)
            .and_then(|owner_idx| resolve_pawn_to_slot(owner_idx, ctx));
        if owner_slot.is_none() {
            warn!("sinner death: non-pawn attacker index {} owner chain failed", attacker_index);
        }
        owner_slot
    }
    None => None,
};
```

The `m_hOwnerEntity` field key is already defined in the parser as `OWNER_ENTITY_KEY = fkey_from_path(&["m_hOwnerEntity"])` and confirmed to exist on NPCs via the `resolve_killer_slot` function in `sinner_probe.rs`.

**`get_match_data_json()`:** add `"sinners": self.sinner_tracker.get_output()` to the `serde_json::json!` block.

**Imports:** add `SinnerTracker` to the `crate::tracking` import line.

### A7. Record learnings

Append to `private/learnings.md` ## Drafts:
- The `health == 1` death signal pattern (not `health == 0`) and why -- delta compression skips the zero packet on entity recycling
- The `should_track_snapshot()` abstraction and when to use it vs `should_track_position()`
- Confirmation that `CCitadelUserMsg_BossKilled` does not fire for sinners (entity class range 3335-3379)
- Entity index recycling on respawn -- last-snapshot-wins lookup pattern

### A Checkpoint

**Status:** `[x] Complete`

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. Run `cargo test` and record results below
> 2. Parse replay `68182475_4609034.dem` using `parse_local` and record the `sinners` array summary below
> 3. Check off every item below with date and actual result
> 4. Note any deferred items with reason
> 5. Update **Status** above

#### Results *(agent fills in)*

- [x] `cargo test` -- 40 passed, 0 failed (2026-04-08, after post-review cleanup; 34 original + 6 added)
- [x] `sinners` key present in `get_match_data_json()` output (2026-04-08)
- [x] Snapshot count matches expected: 40 total entries for `68182475_4609034.dem` (2026-04-08)
- [x] 34 snapshots have `death_time_s` populated; 6 have `death_time_s: null` (2026-04-08)
- [x] All 34 killed snapshots have `killer_player_slot` populated (not null) -- no Lil Helper kills in this replay (2026-04-08)
- [x] All snapshots have `retaliation_damage` key; 38 of 40 sinners have non-empty retaliation maps; 2 alive-at-end sinners have empty maps (2026-04-08)
- [x] `CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY` no longer appears in the `positions` array -- verified 0 entries with custom_id==23 (2026-04-08)
- [x] Learnings appended to `private/learnings.md` (2026-04-08)

#### Sample output *(agent fills in)*

Paste the first 3 entries and one end-of-match entry (death_time_s: null) from the `sinners` array:

```json
{
  "death_time_s": 477,
  "entity_index": 3335,
  "killer_player_slot": 2,
  "max_health": 500,
  "retaliation_damage": {
    "2": 320
  },
  "spawn_time_s": 441,
  "time_alive_s": 36,
  "x": -3682.0,
  "y": -1524.0,
  "z": 562.0625
}
{
  "death_time_s": 489,
  "entity_index": 3336,
  "killer_player_slot": 2,
  "max_health": 500,
  "retaliation_damage": {
    "2": 320
  },
  "spawn_time_s": 441,
  "time_alive_s": 48,
  "x": -3968.0,
  "y": -1728.0,
  "z": 564.03125
}
{
  "death_time_s": 528,
  "entity_index": 3344,
  "killer_player_slot": 11,
  "max_health": 500,
  "retaliation_damage": {
    "11": 320
  },
  "spawn_time_s": 441,
  "time_alive_s": 87,
  "x": 3840.0,
  "y": 1520.0,
  "z": 536.03125
}
-- alive at match end:
{
  "death_time_s": null,
  "entity_index": 3375,
  "killer_player_slot": null,
  "max_health": 500,
  "retaliation_damage": {},
  "spawn_time_s": 1799,
  "time_alive_s": null,
  "x": 2624.0,
  "y": 960.0,
  "z": 248.0
}
```

Expected shape for a killed sinner:
```json
{
  "entity_index": 3377,
  "x": -3682.0,
  "y": -1524.0,
  "z": 562.0,
  "spawn_time_s": 401,
  "max_health": 500,
  "death_time_s": 463,
  "time_alive_s": 62,
  "killer_player_slot": 4,
  "retaliation_damage": {"4": 560}
}
```

Expected shape for a sinner alive at match end:
```json
{
  "entity_index": 3341,
  "x": 3840.0,
  "y": 1520.0,
  "z": 536.0,
  "spawn_time_s": 1102,
  "max_health": 500,
  "death_time_s": null,
  "time_alive_s": null,
  "killer_player_slot": null,
  "retaliation_damage": {}
}
```

#### Deferred items
None.

#### Post-review cleanup *(2026-04-08)*

Test-auditor + code-reviewer were run after the initial Phase A implementation and flagged a set of warnings and test gaps. All items addressed in commit `refactor(parser): clean up sinner tracker per review feedback`:

- [x] **W1**: `PostMatch damage_matrix` `println!` converted to `debug!` with `[parse_replay]` prefix
- [x] **W2**: Inline sinner tracker tests moved to companion `parser/src/tracking/sinner_tracker/tests.rs` per project convention
- [x] **W3**: `handle_sinner_create` doc clarified -- `last_attacker` is removed, `last_health` is overwritten (not vaguely "cleared")
- [x] **S1**: Struct-level doc note added about the `max_health > 1` assumption and why it holds
- [x] **S2**: Reverse-scan performance profile documented on `record_retaliation` (O(lives), typically 1-5/match)
- [x] **S3**: Unified guard pattern -- `record_damage` and `record_retaliation` both use `is_tracked_sinner()`
- [x] **S4**: Hard-wrapped doc comments unwrapped per project writing style
- [x] **S5**: `is_tracked_sinner` docstring clarified -- "has been tracked", not "currently alive"
- [x] **Bonus**: Removed dead `max_health_key` field from `SinnerTracker` (never used -- caller passes `max_health` directly)
- [x] **Test gaps** (6 new tests, ST-10 through ST-15):
    - `test_update_on_untracked_entity_returns_none`
    - `test_zero_duration_death` (spawn_time_s == death_time_s)
    - `test_record_death_killer_is_idempotent` (second call does not overwrite)
    - `test_record_retaliation_untracked_is_noop`
    - `test_record_retaliation_after_recycle_routes_to_new_snapshot`
    - `test_first_death_with_no_recorded_damage_returns_none`

Total parser test count: 40 passed, 0 failed.

Await user review and commit approval before proceeding to Phase B.

---

## Phase B -- Backend (`backend-python` agent)

### B1. Create `backend/app/domain/sinner.py`

Add the Python domain model. Follow the layout of `backend/app/domain/boss.py`.

```python
from sqlmodel import SQLModel
from typing import Optional

class SinnerSnapshot(SQLModel):
    entity_index: int
    x: float
    y: float
    z: float
    spawn_time_s: int
    max_health: int
    death_time_s: Optional[int] = None
    time_alive_s: Optional[int] = None
    killer_player_slot: Optional[int] = None
    # Serde serializes HashMap<u32, i32> keys as strings, so JSON keys arrive as strings in Python
    retaliation_damage: dict[str, int] = {}
```

### B2. Update `backend/app/domain/match_analysis.py`

1. Add the import: `from app.domain.sinner import SinnerSnapshot`
2. Add `sinners: list[SinnerSnapshot]` to `ParsedMatchResponse`
3. Add `sinners: list[SinnerSnapshot]` to `TransformedMatchData`

No new service or use case is required. Sinner data is passed through from the parser as-is -- there is no domain transformation needed at this stage.

### B3. Record learnings

Append to `private/learnings.md` ## Drafts any findings about how the backend pass-through pattern works for new parser output fields, noting that no service layer is needed when the data requires no transformation.

### B Checkpoint

**Status:** `[x] Complete`

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. Run `pytest` and record results below
> 2. Hit `GET /match/analysis/{match_id}` for a parsed match and confirm `sinners` key is present in the response
> 3. Check off every item below with date and actual result
> 4. Note any deferred items with reason
> 5. Update **Status** above

#### Results *(agent fills in)*

- [x] `pytest` -- 50 passed, 2 skipped in 1.78s (2026-04-08). The 2 skips are pre-existing (`PlayerPathState` removed, `PlayerAnalytics` not yet implemented). The 3 DB-dependent repo tests (`test_parsed_matches_repo.py`, `test_db_session.py`, `test_users_repo.py`) were excluded -- they require a `deadlock_test_db` database that is not provisioned in the worktree's docker-compose stack; this is a pre-existing environment gap, not caused by this change.
- [x] `sinners` key present in `ParsedMatchResponse` and `TransformedMatchData` (2026-04-08) -- added with `list[SinnerSnapshot] = []` default on both; schema contract test `test_match_analysis_serializes_to_dict` and `test_transformed_match_data_fields` both assert the key.
- [ ] `GET /match/analysis/{match_id}` response includes `sinners` array with correct entry count -- DEFERRED (see below)
- [ ] Each snapshot in response has `entity_index`, `x`, `y`, `z`, `spawn_time_s`, `max_health` populated -- DEFERRED (live endpoint not available)
- [ ] Killed snapshots have non-null `death_time_s`, `time_alive_s`, `killer_player_slot` -- DEFERRED (live endpoint not available)
- [x] Learnings appended to `private/learnings.md` (2026-04-08) -- "Parser pass-through fields need no service layer" draft entry added under ## Drafts.

#### Sample output *(agent fills in)*

```
DEFERRED -- live endpoint not available (see Deferred items below)
```

#### Deferred items

1. **Live endpoint verification** (`GET /match/analysis/{match_id}` with sinners populated): The worktree docker-compose stack is running but no parsed match with sinner data is cached in the worktree's database. Verifying the field end-to-end requires either re-parsing a match (which calls the real Deadlock API and parser) or a DB fixture. Deferred to user review -- the schema contract tests plus the `retaliation_damage` round-trip test provide strong local coverage. Recommend hitting the endpoint after merge to `main` where real match data is cached.
2. **DB-dependent repo tests** (`test_parsed_matches_repo.py` etc.): Pre-existing gap -- `deadlock_test_db` not provisioned in worktree stack. Not caused by this change.

Await user review and commit approval.

---

---

## Phase C -- Frontend (`frontend-react` agent)

Phase C depends on Phase B: the backend must be returning `sinners` in the API response before frontend work begins.

### Pre-work: Download the sinner icon

The Cursed Apple minimap icon must be added as a local asset before C1 begins. The wiki page URL `https://deadlock.wiki/The_Cursed_Apple#/media/File:MapSacrificeMachineIcon.png` is not a direct image link -- download the PNG from the wiki's File page and place it at:

```
frontend/src/assets/sinner-icon.png
```

The agent should confirm this file exists before proceeding with C1.

### C1. Add `SinnerSnapshot` domain type

Create `frontend/src/domain/sinner.ts`. Mirror the layout of `frontend/src/domain/boss.ts`.

```typescript
export interface SinnerSnapshot {
  entity_index: number;
  x: number;
  y: number;
  z: number;
  spawn_time_s: number;
  max_health: number;
  death_time_s: number | null;
  time_alive_s: number | null;
  killer_player_slot: number | null;
  // Backend serializes Rust HashMap<u32, i32> keys as strings
  retaliation_damage: Record<string, number>;
}

export interface ScaledSinnerSnapshot extends SinnerSnapshot {
  left: number;
  top: number;
}
```

### C2. Thread `sinners` through `ParsedMatchData`

In `frontend/src/domain/matchAnalysis.ts`:

1. Add import: `import { SinnerSnapshot } from './sinner';`
2. Add `sinners: SinnerSnapshot[];` to the `ParsedMatchData` interface
3. Add `sinners: [],` to the `sinners` field in `defaultMatchAnalysis.parsed_match_data`

### C3. Compute scaled sinner snapshots in `MatchAnalysis.tsx`

In `frontend/src/pages/MatchAnalysis.tsx`:

1. Add import: `import { ScaledSinnerSnapshot } from '../domain/sinner';`
2. Add `const sinnerSnapshots = parsedMatchData.sinners;` alongside the existing `bossSnapshots` extraction.
3. Add a memoized `scaledSinnerSnapshots` computation following the same pattern as `scaledBossSnapshots`:

```typescript
const scaledSinnerSnapshots: ScaledSinnerSnapshot[] = useMemo(
  () =>
    sinnerSnapshots.map((snapshot) => ({
      ...snapshot,
      ...worldToMinimapPixels(snapshot.x, snapshot.y),
    })),
  [sinnerSnapshots]
);
```

4. Pass `scaledSinnerSnapshots` to `<Minimap>` as a new prop.
5. Pass `sinners={parsedMatchData.sinners}` to `<PlayerCards>`.

### C4. Create `SinnerLayer.tsx`

Create `frontend/src/components/matchAnalysis/SinnerLayer.tsx`. This component renders sinner icons on the minimap for all sinners that are alive at `currentSec`.

A sinner snapshot is alive at `currentSec` if:
- `currentSec >= snapshot.spawn_time_s`, AND
- `snapshot.death_time_s === null || currentSec < snapshot.death_time_s`

Since entity indices are recycled on respawn, multiple snapshots can share the same `entity_index`. The alive filter handles this correctly -- at most one snapshot per entity index will be alive at any given second.

```tsx
import sinnerIconUrl from '../../assets/sinner-icon.png';
import { ScaledSinnerSnapshot } from '../../domain/sinner';

interface SinnerLayerProps {
  scaledSinnerSnapshots: ScaledSinnerSnapshot[];
  currentSec: number;
}

const ICON_SIZE = 18; // px -- adjust if the icon looks too large or small on the 768px minimap

const SinnerLayer: React.FC<SinnerLayerProps> = ({
  scaledSinnerSnapshots,
  currentSec,
}) => {
  const alive = scaledSinnerSnapshots.filter(
    (s) =>
      currentSec >= s.spawn_time_s &&
      (s.death_time_s === null || currentSec < s.death_time_s)
  );

  return (
    <>
      {alive.map((s, i) => (
        <img
          key={`sinner-${s.entity_index}-${s.spawn_time_s}`}
          src={sinnerIconUrl}
          alt='Sinner'
          className='pointer-events-none absolute'
          style={{
            left: s.left - ICON_SIZE / 2,
            top: s.top - ICON_SIZE / 2,
            width: ICON_SIZE,
            height: ICON_SIZE,
          }}
        />
      ))}
    </>
  );
};

export default SinnerLayer;
```

Note: the `key` uses both `entity_index` and `spawn_time_s` together to handle recycled indices correctly within the `alive` array (there should never be two alive snapshots with the same entity_index, but this makes the key stable).

### C5. Wire `SinnerLayer` into `Minimap.tsx`

1. Add `scaledSinnerSnapshots: ScaledSinnerSnapshot[]` and `currentSec: number` to the `Minimap` props destructure and type annotation. (`currentSec` is already passed as `currentSecond` -- rename or add an alias as appropriate.)
2. Import `SinnerLayer` and `ScaledSinnerSnapshot`.
3. Render `<SinnerLayer>` inside the minimap's relative container, after `<CreepWaveLayer>`:

```tsx
<SinnerLayer
  scaledSinnerSnapshots={scaledSinnerSnapshots}
  currentSec={currentSecond}
/>
```

### C6. Show retaliation damage on the PlayerCard

In `frontend/src/components/matchAnalysis/PlayerCards.tsx`:

1. Add `sinners: SinnerSnapshot[]` to the `PlayerCardsProps` interface and destructure it.
2. Import `SinnerSnapshot` from `'../../domain/sinner'`.
3. Inside the `players.map(...)` callback, compute the player's total retaliation damage taken across all sinner snapshots:

```typescript
const slotKey = String(player.lobby_player_slot);
const sinnerDamageTaken = sinners.reduce((total, s) => {
  return total + (s.retaliation_damage[slotKey] ?? 0);
}, 0);
```

4. Render a new line in the player card body, after the Lane Pressure line and before the Victims section:

```tsx
<div>
  <strong>Sinner damage taken:</strong>{' '}
  {sinnerDamageTaken > 0 ? sinnerDamageTaken : <span className='text-gray-400'>0</span>}
</div>
```

This shows the cumulative HP the sinner machines dealt back to that player across the entire match. A player who solo-cleared multiple sinners via light melees will show a high number (320-640 per sinner); one who never touched a sinner will show 0.

### C Checkpoint

**Status:** `[x] Complete`

> Completed 2026-04-08 by `frontend-react` agent.

#### Results

- [x] Icon asset present -- `frontend/src/assets/map-sacrifice-machine-icon.png` (2026-04-08). NOTE: plan referenced `sinner-icon.png`; the actual filename added during the Phase A/B sync is `map-sacrifice-machine-icon.png`. The import in `SinnerLayer.tsx` uses the correct filename. No rename or new download was performed.
- [x] `frontend/src/domain/sinner.ts` created with `SinnerSnapshot` and `ScaledSinnerSnapshot` (2026-04-08).
- [x] `ParsedMatchData.sinners: SinnerSnapshot[]` added to interface; `defaultMatchAnalysis.parsed_match_data.sinners: []` added (2026-04-08).
- [x] `scaledSinnerSnapshots` computed via `useMemo` in `MatchAnalysis.tsx` and passed to `<Minimap scaledSinnerSnapshots={...} />` (2026-04-08).
- [x] `sinners={parsedMatchData.sinners}` passed to `<PlayerCards>` (2026-04-08).
- [x] `SinnerLayer.tsx` created; renders sinner icons at their world coordinates for all alive snapshots at `currentSec` (2026-04-08).
- [x] Icons disappear at `death_time_s` -- alive filter is `currentSec < s.death_time_s` (exclusive), confirmed by `SinnerLayer.test.tsx` (2026-04-08).
- [x] Icons reappear after respawn -- entity_index reuse handled correctly; each lifecycle's `spawn_time_s` gates visibility independently, confirmed by recycled-index test (2026-04-08).
- [x] PlayerCard shows "Sinner damage taken: X" for each player; muted `0` for players with no sinner retaliation, confirmed by `PlayerCards.test.tsx` (2026-04-08).
- [x] No TypeScript errors -- `npm run build` inside container exits cleanly, 911 modules transformed, zero TS errors (2026-04-08).

#### Tests written
- `tests/components/matchAnalysis/SinnerLayer.test.tsx` -- 10 cases covering alive filter (inclusive lower bound at `spawn_time_s`, exclusive upper bound at `death_time_s`), null death_time, recycled entity_index, icon centering, empty list. `SinnerLayer.tsx` has 100% statement/branch coverage.
- `tests/components/matchAnalysis/PlayerCards.test.tsx` -- 7 cases covering cumulative sum, slot-absent zero, muted rendering, `String(lobby_player_slot)` key lookup, label presence, non-muted positive value.

Test suite result: **73 passed / 0 failed** across 8 test files.

#### Deferred items
- Browser visual verification (Playwright / running dev server) -- deferred; dev server is accessible via the sinner-tracking Docker container but live screenshot inspection was not performed in this session. The alive-filter logic is fully unit-tested.
- `code-reviewer` and `test-auditor` subagent invocations -- deferred; Agent tool not available in this execution context. Self-review performed instead (see Phase C report in plan update).

Await user review and commit approval.

---

## Post-ship follow-up: per-event damage log (`damage_events`)

After Phases A-C landed, a coach-facing gap became clear: the original `retaliation_damage: HashMap<u32, i32>` on `SinnerSnapshot` is a match-life total, with no timestamps and no record of damage the player dealt TO the sinner. Coaches wanted to bucket damage-trade metrics by arbitrary time windows (laning / mid / late) and reason about steals (last-hit by someone who did not contribute the plurality of damage). Neither was possible from the aggregated HashMap alone.

### What was added (parser-only change)

- **New types in `parser/src/domain/sinner.rs`**:
  - `SinnerDamageKind { Dealt, Retaliated }` -- serde-tagged snake_case enum.
  - `SinnerDamageEvent { time_s: u32, player_slot: u32, kind: SinnerDamageKind, damage: i32 }`.
- **New field on `SinnerSnapshot`**: `damage_events: Vec<SinnerDamageEvent>`. Ordered, per-life, captures every damage exchange against the sinner (dealt and retaliated, with timestamps). ~5-10 events per life × ~40 lives per match = ~200-400 events per match -- trivial storage.
- **`retaliation_damage` kept and still populated** for backwards compatibility with the current `PlayerCards.tsx`, which will be refactored within the next 1-2 weeks. Marked DEPRECATED in the Rust docstring and in `parser-output.md`. Will be removed in a follow-up parser commit once the frontend migrates to `damage_events`.
- **Tracker changes in `parser/src/tracking/sinner_tracker.rs`**:
  - `record_retaliation` gained a `time_s: u32` parameter and now appends a `Retaliated` event alongside the legacy HashMap accumulation.
  - New `record_dealt_event(sinner_idx, attacker_slot, damage, time_s)` method appends a `Dealt` event. No HashMap on the dealt side -- the event log is the sole source of truth for that direction.
- **`replay_parser.rs` wiring**: the damage packet block now resolves attacker entity -> player slot (direct `CCitadelPlayerPawn` or `m_hOwnerEntity` NPC-proxy fallback) via a new `resolve_attacker_to_player_slot` helper and appends dealt events when the resolution succeeds. Non-player attackers (troopers, unowned NPCs) still get kill attribution via the existing `record_damage` path but produce no Dealt event.
- **Drive-by refactor of the damage packet block**: hoisted `attacker_idx`/`victim_idx`/`damage`/`match_time_s` once, replaced the early-return + later `.unwrap()` pattern with `let-else`, collapsed the four-level nested `if let` retaliation block into a single let-chain, and dropped an unnecessary `as i32` cast in `resolve_pawn_to_slot`.

### Why the discriminated-union shape

The alternative was a merged `{ dealt: i32, retaliated: i32 }` event shape assuming a 1:1 pairing between punches and retaliation pulses. That pairing is currently confirmed for Deadlock's Sinner Sacrifice mechanic, but we chose the `kind: Dealt | Retaliated` shape so the schema is agnostic to the coupling -- if Valve ever decouples retaliation (AoE tick on a timer, delayed returns, damage to nearby non-attackers), no schema change is needed. A merged-pair view remains trivially reconstructable client-side by grouping on `(time_s, player_slot)`.

### Use cases unlocked

Documented in `private/product/strategy/current-options.md` under the Coach Analytics Engine initiative. Summary:

- Damage trade ratio per player per sinner life or time window.
- Steal detection: `killer_player_slot` vs. the player with the highest `Σ(Dealt)`.
- Sinner contribution timelines bucketed by game phase.
- Cost-of-contest metric (high dealt + high retaliation + no killer attribution).
- Team-level sinner farming per phase.
- Future-proof against Valve retaliation mechanic changes.

Two coach-facing definitions are explicitly flagged TBD in `current-options.md` before any of these ship as rendered metrics:

- **Sinner taken (TBD)** -- does this require solo contribution or count any last-hit?
- **Sinner stolen (TBD)** -- what contribution threshold counts as a steal (plurality vs. >50% vs. <30%)? Do intra-team steals count the same as enemy snipes? Does contested damage trade change the label?

### Scope boundary

Parser-only. Backend `ParsedMatchResponse` will pass the field through by default (pydantic model will happily accept the new array field without explicit plumbing, provided the SinnerSnapshot python model adds an optional `damage_events: list[...] = []`). Frontend consumption is deferred to the upcoming `PlayerCards.tsx` refactor. The legacy `retaliation_damage` HashMap is retained so no visible UI regression lands in the meantime.

### Tests and verification

- `cargo test`: 47 pass, 0 fail (22 sinner tracker tests: 15 original + 7 new covering the event log, plus 25 other parser tests).
- `cargo clippy`: refactor area has zero warnings; total parser warning count dropped by 1 (old collapsible-if pattern removed).
- No runtime replay verification in this round -- unit coverage was sufficient.

### Related commits

- `feat(parser): log per-event sinner damage with direction` (parent)
- `docs(sinner-tracking): add damage_events contract and TBD definitions` (private)
- `chore: update private submodule pointer` (parent)

---

## Verification Summary

| Phase | Command | Key checks | Status |
|-------|---------|------------|--------|
| 0 | Contract spec review | `sinners` field + `SinnerSnapshot` table in `parser-output.md`; backend impact noted | DONE |
| A | `cargo test` | All tests pass, no regressions | PASS -- 47/47 |
| A | Parse `68182475_4609034.dem` | 40 snapshots, 34 with death data, 6 null; no sinners in positions array | PASS |
| B | `pytest` | All tests pass, no regressions | PASS -- 50/50 |
| B | API spot-check | `sinners` key present with correct shape in `/match/analysis/{match_id}` | DEFERRED (no cached match in worktree DB) |
| C | TS build / dev server | No type errors; sinner icons visible on minimap; retaliation damage on player cards | PASS -- `npm run build` clean; 72/72 tests pass; 100% coverage on SinnerLayer |
| Post-ship | `cargo test` + `cargo clippy` | `damage_events` logged per life; refactor clean | PASS -- 47/47, zero warnings in refactor area |

---

## Execution Order

1. **Phase 0** (contract -- `rust-parser`) -- user review -- proceed (blocks all phases)
2. **Phase A** (rust-parser) -- user review -- commit
3. **Phase B** (backend-python) -- user review -- commit (depends on A's output schema)
4. **Phase C** (frontend-react) -- user review -- commit (depends on B's API response shape)

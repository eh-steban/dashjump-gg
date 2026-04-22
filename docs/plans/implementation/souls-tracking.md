# Plan: Souls Tracking

## Context

Track currency (souls) data from Deadlock replays and visualize it as a running total per player per second. Per-source breakdown (lane creeps, denies, kills) is included at no extra cost and directly feeds the active "lane priority tracking" initiative.

**Spike:** `private/plans/spikes/currency-changed-haste-api-compat.md` -- resolves all API unknowns.

---

## Confirmed: `CCitadelUserMessageCurrencyChanged` Field Reality

From generated types (`parser/target/debug/build/valveprotos-0c71e7db1e5291c4/out/deadlock.rs:2461-2480`):

| Field | Type | Use |
|-------|------|-----|
| `entindex_hero_pawn` | `Option<i32>` | Player identifier (NOT `userid`) |
| `currency_type` | `Option<i32>` | Always 0 -- single currency confirmed |
| `currency_source` | `Option<i32>` | EGoldSource (1=Players, 2=LaneCreeps, 7=Denies, ...) |
| `delta` | `Option<i32>` | Signed change; positive=earn, negative=spend |
| `entindex_victim` | `Option<i32>` | Set when source=Players(1) -- kill bounty target |
| `ability_id` | `Option<u32>` | Set on ability-related gold |

**`new_value` does not exist.** Balance must be accumulated from deltas.

---

## Part 1: Update Probe Binary

The probe (`parser/src/bin/probe_currency_changed.rs`) was written for `blukai/haste` (sync Visitor). The current dependency is `deadlock-api/haste` (async Visitor).

**Two mechanical changes required:**

1. Add `type Error = anyhow::Error;` to the Visitor impl
2. Change all `fn on_packet/on_entity/on_tick_end/on_cmd` to `async fn`
3. Fix player name lookup: `get_value::<Box<[u8]>>(&PLAYER_NAME_KEY).map(|b| String::from_utf8_lossy(&b).into_owned())` (pattern from `replay_parser.rs:205-208`)

**Run after fixing:**
```bash
docker-compose run --rm dashjump-parser cargo run --bin probe_currency_changed -- /parser/src/replays/55423930_379917638.dem
```

**Delete after probe output is reviewed.** Remove `probe_currency_changed.rs`; no Cargo.toml entry needed (auto-discovered from `src/bin/`).

### CHECKPOINT -- review probe output before Part 2

Present: currency_type distribution, per-source earn tallies, per-second balance sample for one player.

---

## Part 2: Implementation

### Parser: New `souls_tracker.rs`

Pattern: `parser/src/tracking/boss_tracker.rs` (per-second snapshots).

**File:** `parser/src/tracking/souls_tracker.rs`

**Data model:**

```rust
// domain/souls.rs
pub struct SoulsSnapshot {
    // Index = match_sec; value per player_slot
    pub balance: HashMap<u32, i64>,          // accumulated from deltas; no new_value field
    pub earned_by_source: HashMap<u32, HashMap<u8, i64>>,  // player_slot -> source -> souls earned
}

pub struct KillBountyEvent {
    pub match_sec: u32,
    pub earning_player_slot: u32,
    pub victim_entindex: i32,   // resolve to player_slot in tracker or leave raw for backend
    pub souls_earned: i64,
}

pub struct SoulsData {
    pub timeline: Vec<SoulsSnapshot>,        // per-second, match-relative
    pub kill_bounties: Vec<KillBountyEvent>, // kill gold events with victim identity
}
```

**Tracker state machine (`souls_tracker.rs`):**

```rust
pub struct SoulsTracker {
    balance: HashMap<u32, i64>,                    // player_slot -> current balance
    earned_this_sec: HashMap<u32, HashMap<u8, i64>>, // player_slot -> source -> earn accumulator
    timeline: Vec<SoulsSnapshot>,
    kill_bounties: Vec<KillBountyEvent>,
}

impl SoulsTracker {
    pub fn handle_currency_changed(
        &mut self,
        player_slot: u32,
        currency_source: i32,
        delta: i32,
        entindex_victim: Option<i32>,
        match_sec: u32,
    )

    pub fn build_snapshot(&mut self, match_sec: u32)  // carry-forward into timeline

    pub fn get_output(self) -> SoulsData
}
```

**Key logic:**
- `balance[slot] += delta` on every event (both earn and spend)
- Only accumulate `earned_by_source` for `delta > 0`
- For `delta > 0 && currency_source == 1 (Players)`: append a `KillBountyEvent` with `entindex_victim`
- `build_snapshot` carries forward the last balance into the timeline for seconds with no events

**Integration in `replay_parser.rs`:**
- Subscribe to `CitadelUserMessageIds::KEUserMsgCurrencyChanged` in `async fn on_packet`
- Resolve `entindex_hero_pawn -> m_hOwnerEntity -> controller -> m_unLobbyPlayerSlot` using existing `ehandle_to_index` pattern (`replay_parser.rs:193-229`)
- Call `build_snapshot(match_window)` in `on_tick_end` after `boss_tracker.build_health_window`
- Add `"souls": souls_tracker.get_output()` to `get_match_data_json()`

### Backend

No changes needed for the initial feature. Souls data passes through the existing `/parse` endpoint JSON blob as-is.

**Future:** `backend/app/services/lane_pressure_service.py` may aggregate `earned_by_source[LaneCreeps]` for lane pressure scoring -- that work belongs to the lane priority initiative, not here.

### Frontend: Souls Timeline Visualization

**New component:** `frontend/src/components/matchAnalysis/SoulsTimeline.tsx`

Display: running balance per player over match time (line chart, one line per player, colored by team). Balance is the primary view -- coaches can see at a glance who has a gold lead and when leads shifted.

**Integration:** `frontend/src/pages/MatchAnalysis.tsx` -- add alongside existing visualizations.

The per-source breakdown (LaneCreeps, Players, Denies) is available in the data but is not required for the first visualization. Expose it in the JSON output; visualizing breakdowns is a Phase 2 addition once coaches confirm what's useful.

---

## Files to Create/Modify

| File | Action | Notes |
|------|--------|-------|
| `parser/src/bin/probe_currency_changed.rs` | Edit | Fix async Visitor API + Box<[u8]> player name |
| `parser/src/domain/souls.rs` | Create | `SoulsData`, `SoulsSnapshot`, `KillBountyEvent` |
| `parser/src/domain/mod.rs` | Edit | Register souls module |
| `parser/src/tracking/souls_tracker.rs` | Create | Tracker following boss_tracker.rs pattern |
| `parser/src/tracking/mod.rs` | Edit | Register souls_tracker module |
| `parser/src/replay_parser.rs` | Edit | Subscribe to CurrencyChanged, call snapshot, expose output |
| `frontend/src/components/matchAnalysis/SoulsTimeline.tsx` | Create | Running balance line chart |
| `frontend/src/pages/MatchAnalysis.tsx` | Edit | Mount SoulsTimeline |

---

## What We're Collecting (and Why)

| Data | Source field | Product alignment |
|------|-------------|-------------------|
| Balance per player per second | `delta` accumulation | Core goal -- souls total over time |
| Souls by source per second | `currency_source` grouping | Lane priority -- who last-hits / denies more |
| Kill bounties with victim | `entindex_victim` when source=1 | Hero matchup -- gold earned from specific opponents |

**Deferred (not in scope here):**
- `RecentDamageSummary.lost_gold` death annotations -- fight classification initiative
- `HeroKilled` subscription for death timeline -- fight classification initiative
- `AbilitiesChanged` / `ItemPurchaseNotification` for spend annotations -- Phase 2

---

## Key Reference Files

- `private/plans/spikes/currency-changed-haste-api-compat.md` -- type verification findings
- `parser/src/bin/probe_currency_changed.rs` -- probe to run first
- `parser/src/tracking/boss_tracker.rs` -- snapshot pattern to follow
- `parser/src/replay_parser.rs:193-229` -- player slot resolution pattern
- `parser/src/replay_parser.rs:299-513` -- async Visitor pattern
- `private/specs/citadel-messages-reference.md` -- EGoldSource enum values (note: `CurrencyChanged` entry has errors; see spike)

---

## Verification

1. **Probe:** Runs, prints currency_type distribution (expect only one value), per-source earn tallies, per-second balance sample for one player
2. **Parser unit tests:** `SoulsTracker::handle_currency_changed` -- earn accumulation, spend not accumulated in `earned_by_source`, carry-forward snapshot, kill bounty recorded only for source=1
3. **Integration:** Parse a real replay, verify `souls.timeline` contains per-second per-player data; verify `souls.kill_bounties` has entries with non-null victim indices
4. **Frontend:** SoulsTimeline renders correctly at `/match/:id` with one line per player

---

## Parking Lot

These are related ideas that are real opportunities but not in scope for this feature:

| Idea | Why deferred |
|------|-------------|
| Unsecured souls (floating after death) | Requires entity-level tracking (not a user message); separate parser work |
| Souls lost on death annotation | `RecentDamageSummary.lost_gold` -- fight classification initiative owns this |
| Item purchase timeline overlay | `ItemPurchaseNotification` not in generated types; needs investigation |
| Per-minute reconciliation via GoldHistory | Low value -- good sanity check but not needed for feature |
| Lane pressure scoring from LaneCreeps gold | Backend aggregation layer -- belongs to lane priority initiative |

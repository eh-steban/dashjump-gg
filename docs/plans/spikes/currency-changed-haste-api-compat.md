# CurrencyChanged Haste API Compatibility Spike

## Context

The souls-tracking implementation plan was written against `blukai/haste` (sync Visitor, different proto generation). The parser now depends on `deadlock-api/haste` (async Visitor). Before implementation starts, we need to confirm what fields exist on `CCitadelUserMessageCurrencyChanged` and related messages, whether the types match what was assumed, and whether we can collect more data that aligns with the Q1/Q2 coach analytics roadmap.

---

## Question

Does the current `deadlock-api/haste` dependency change the available fields or API patterns for implementing souls tracking, and are there related messages we should collect from that align with the active product strategy?

---

## Assumptions

### To Validate

- [x] `new_value: uint32` field exists on `CCitadelUserMessageCurrencyChanged` -- *How to check: generated types in `parser/target/debug/build/valveprotos-*/out/deadlock.rs`*
- [x] `userid` is the player identifier field -- *How to check: same generated types*
- [x] Probe binary compiles with current haste (async Visitor pattern) -- *How to check: compare probe Visitor impl against replay_parser.rs impl*
- [x] `CCitadelUserMsgAbilitiesChanged.purchaser_player_slot` -- field name correct -- *How to check: generated types*

### Accepted (not tested here)

- `entindex_hero_pawn -> m_hOwnerEntity -> controller -> m_unLobbyPlayerSlot` resolve chain still works -- *Risk if wrong: player attribution breaks*
- EGoldSource enum values (1=Players, 2=LaneCreeps, 7=Denies, etc.) are unchanged -- *Risk if wrong: source labels mislabeled in output*

---

## Agent & Timebox

**Agent:** haste-expert / manual (resolved from generated types directly)
**Timebox:** 30 minutes

---

## Research Standards

Follow `.claude/rules/research.md` for confidence labels, citation format, and scope discipline.

---

## Investigation Approach

1. Read `parser/target/debug/build/valveprotos-*/out/deadlock.rs` at the struct definitions for all relevant messages
2. Compare `probe_currency_changed.rs` Visitor impl against `replay_parser.rs` to identify API surface mismatches
3. Cross-reference available fields against Q1/Q2 product options in `private/product/strategy/current-options.md`

---

## Findings

**Answer:** Three assumptions are invalidated -- `new_value` does not exist, `userid` is not a field (it is `entindex_hero_pawn`), and the probe binary is incompatible with the current async Visitor API. However, the core tracking approach is sound. Additionally, the CurrencyChanged `currency_source` field directly feeds the active "lane priority tracking" initiative at no extra implementation cost.

---

### Message Type Verification (confirmed from generated types)

**Source:** `parser/target/debug/build/valveprotos-0c71e7db1e5291c4/out/deadlock.rs`

#### `CCitadelUserMessageCurrencyChanged` (ID 345) -- lines 2461-2480

| Field | Type | Tag | Notes |
|-------|------|-----|-------|
| `entindex_hero_pawn` | `Option<i32>` | 1 | Player identifier -- NOT `userid` |
| `currency_type` | `Option<i32>` | 2 | |
| `currency_source` | `Option<i32>` | 3 | EGoldSource enum |
| `delta` | `Option<i32>` | 4 | Signed; positive=earn, negative=spend |
| `notification` | `Option<bool>` | 5 | |
| `entindex_victim` | `Option<i32>` | 6 | Set on kill/assist sources |
| `victim_pos` | `Option<CMsgVector>` | 7 | |
| `playsound` | `Option<i32>` | 8 | |
| `ability_id` | `Option<u32>` | 9 | |

**Critical: `new_value` is absent.** Balance must be accumulated from deltas.
**Critical: `userid` is absent.** Use `entindex_hero_pawn` for player resolution.

#### `CCitadelUserMsgGoldHistory` (ID 313) -- lines 2127-2153

| Field | Type | Notes |
|-------|------|-------|
| `entindex_player` | `Option<i32>` | |
| `minute_records` | `Vec<MinuteRecord>` | Each has `match_minute` + `Vec<GoldRecord>` |
| `GoldRecord.currency_source` | `Option<i32>` | EGoldSource |
| `GoldRecord.gold` | `Option<i32>` | Souls earned that minute from that source |
| `GoldRecord.events` | `Option<i32>` | Transaction count for that minute/source |

Confirmed match for reference doc. Per-minute resolution -- use as sanity check against CurrencyChanged accumulation, not as primary data source.

#### `CCitadelUserMsgHeroKilled` (ID 319) -- lines 2295-2308

| Field | Type | Notes |
|-------|------|-------|
| `entindex_victim` | `Option<i32>` | |
| `entindex_inflictor` | `Option<i32>` | |
| `entindex_attacker` | `Option<i32>` | |
| `entindex_assisters` | `Vec<i32>` | |
| `entindex_scorer` | `Option<i32>` | |
| `respawn_reason` | `Option<i32>` | |

No `match_time` or `gametime` field -- timestamp must be derived from tick. No souls amount -- that comes from `CurrencyChanged.entindex_victim`.

#### `CCitadelUserMsgRecentDamageSummary` (ID 310) -- lines 2024-2042

| Field | Type | Notes |
|-------|------|-------|
| `player_slot` | `Option<i32>` | |
| `damage_records` | `Vec<DamageRecord>` | |
| `start_time` | `Option<f32>` | |
| `end_time` | `Option<f32>` | |
| `total_damage` | `Option<i32>` | |
| `lost_gold` | `Option<i32>` | **Souls lost on death -- confirmed present** |
| `modifier_records` | `Vec<ModifierRecord>` | |

`lost_gold` confirmed: `Option<i32>` at tag 6.

#### `CCitadelUserMsgAbilitiesChanged` (ID 309) -- lines 1938-1951

| Field | Type | Notes |
|-------|------|-------|
| `entindex_purchaser` | `Option<i32>` | **NOT `purchaser_player_slot`** -- reference doc wrong |
| `entindex_ability` | `Option<i32>` | Entity index |
| `ability_id` | `Option<u32>` | |
| `change` | `Option<i32>` | EPurchased=0, EUpgraded=1, ESold=2, ... |

**`ItemPurchaseNotification` (ID 360):** No struct found in generated types -- message fires in the demo stream but may not be parsed by valveprotos. Cannot be used without further investigation.

#### `CCitadelUserMsgDeathReplayData` (ID 333) -- lines 2401-2408

| Field | Type | Notes |
|-------|------|-------|
| `killer_scorer` | `Option<i32>` | |
| `killer_inflictor` | `Option<i32>` | |
| `damage_summary` | `Option<CCitadelUserMsgRecentDamageSummary>` | Embeds RecentDamageSummary (with `lost_gold`) |

---

### API Compatibility

**Probe binary is incompatible with current haste on two counts:**

1. **Sync Visitor:** `probe_currency_changed.rs` uses sync `fn on_packet`. Current haste requires `async fn on_packet` with `type Error = anyhow::Error`. Pattern from `replay_parser.rs:299-513`.

2. **Player name type:** Probe calls `get_value::<String>(&PLAYER_NAME_KEY)` -- incompatible. Current pattern: `get_value::<Box<[u8]>>(...).map(|b| String::from_utf8_lossy(&b).into_owned())` (`replay_parser.rs:205-208`).

Both are mechanical updates -- no logic changes required.

---

### Product Alignment Analysis

Active Q1/Q2 initiatives from `private/product/strategy/current-options.md`:

| Initiative | Available Signal | Source | Effort to Collect |
|------------|-----------------|--------|-------------------|
| Lane priority tracking | `currency_source=2` (LaneCreeps) per-player per-second | `CurrencyChanged.delta` filtered by source | **Zero extra effort** -- already iterating all events |
| Lane priority tracking | Deny tracking | `currency_source=7` (Denies) | **Zero extra effort** -- same |
| Hero matchup by player | Kill bounty: who earned souls from killing whom | `CurrencyChanged.entindex_victim` when source=1 (Players) | **Low effort** -- entindex_victim already in the message |
| Fight classification | Death timing with souls annotation | `RecentDamageSummary.lost_gold` + match_sec | Medium -- separate message subscription |
| Fight classification | Kill events with game time | `HeroKilled` + tick->match_sec | Medium -- separate subscription, no souls delta |

**What aligns with current roadmap and is low-effort to include in v1:**
- Per-source souls accumulation (LaneCreeps, Denies, Players, all others) -- directly feeds lane priority and matchup analytics. Cost: group by `currency_source` before writing to the tracker output.
- `entindex_victim` capture on kill events -- feeds hero matchup. Cost: store `entindex_victim` when `delta > 0 && currency_source == 1`.

**What to defer (not in current coach priorities):**
- `RecentDamageSummary.lost_gold` annotation -- fight classification work, separate initiative
- `AbilitiesChanged` / `ItemPurchaseNotification` -- ability/item build analytics, Phase 2+
- `HeroKilled` subscription for kill timeline -- fight classification, separate initiative; the kill signal is already reachable via `CurrencyChanged.entindex_victim`

---

### Assumptions Check

- [x] `new_value: uint32` field exists -- **invalidated** -- absent from generated struct (9 fields, no new_value)
- [x] `userid` is the player identifier -- **invalidated** -- field is `entindex_hero_pawn: Option<i32>`
- [x] Probe compiles with current haste -- **invalidated** -- sync Visitor + String player name incompatible
- [x] `AbilitiesChanged.purchaser_player_slot` field name -- **invalidated** -- actual field is `entindex_purchaser`

**Reference doc errors to fix in `citadel-messages-reference.md`:**
- CurrencyChanged: remove `userid: int32`, add `entindex_hero_pawn: int32`; remove `new_value: uint32`
- AbilitiesChanged: `purchaser_player_slot: int32` → `entindex_purchaser: int32` (entity index, not slot)

---

## Learnings Output

- [x] Draft entry appended to `private/learnings.md` ## Drafts
- [ ] Follow-up spikes: none -- findings are sufficient to unblock implementation
- [x] `citadel-messages-reference.md` needs corrections (see above)

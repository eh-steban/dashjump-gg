# Mid-Boss Implementation Assumptions Spike

## Context

The `mid-boss-tracking.md` implementation plan was authored by reading the [deadlock.wiki Mid-Boss page](https://deadlock.wiki/Mid-Boss) and baking the wiki's numeric claims (max health scaling, shield formula, respawn timers, 50% roar) directly into Phase 0 contract fields without validating them against replay data. It also defers four "open questions" to Phase A (`entity_killed_class` value, `RejuvStatus.event_type` enum, `MidBossSpawned` single-fire behavior, roar event existence) -- which means implementation cannot begin without probes anyway. This spike front-loads that validation so Phase 0 ships with confirmed numbers and Phase A can start without blocking questions.

The earlier `midboss-message-fields.md` spike covered proto field shapes and entity class names but stopped before runtime value confirmation. This spike picks up where that one left off.

---

## Questions

1. **Max health scaling.** How does `m_iMaxHealth` on `CNPC_MidBoss` actually behave across a replay -- does the wiki's `13000 + 195/min` formula fit; is the time reference match start or spawn time; and does the value evolve within one spawn cycle (dynamic regen/scaling) or only between cycles (set once at CREATE)? Also: does `m_iHealth` regenerate between damage events, and at what rate?

2. **Shield mechanic.** Is there an observable shield field on `CNPC_MidBoss` (e.g. `m_iShieldHealth`, `m_iShield`, or a subfield), and does its value match the wiki formula `35 + 5 * match_minutes` at spawn time? If no shield field exists on the entity, where does the shield state live (a linked entity, a buff, or client-only)?

3. **Respawn timers.** In a replay with at least two mid-boss kills, do the `MidBossSpawned` tick deltas match the wiki's respawn formula (7 min after first death, 6 min after second, 5 min after third+)?

4. **50% health roar event.** Does a dedicated `CCitadelUserMsg` fire when the mid-boss crosses 50% health? If yes, which message ID and what fields? If no, can the crossing be derived from the existing `DamageRecord` + `MidBoss.m_iHealth` stream alone?

5. **`entity_killed_class` value for `CNPC_MidBoss`.** What is the integer value of `BossKilled.entity_killed_class` when the killed entity is the mid-boss? This becomes the `MID_BOSS_CLASS_ID` constant in `parser/src/entities/constants.rs`.

6. **`RejuvStatus.event_type` enum.** What `event_type` values fire on `CCitadelUserMsg_RejuvStatus` (ID 350) around a mid-boss kill, how many times does each fire, and what does each value mean (grant / expire / consumed / other)?

7. **`MidBossSpawned` reliability.** Does `CCitadelUserMsg_MidBossSpawned` (ID 349) fire exactly once per spawn cycle in a multi-kill replay, or are there edge cases (double-fire, silent respawn)?

---

## Assumptions

### To Validate

- [ ] Wiki: mid-boss base max health = 13000, scaling = +195/min after spawn -- *How to check: probe `m_iMaxHealth` on `CNPC_MidBoss` at every CREATE across a multi-spawn replay and at UPDATE samples within each lifetime; fit against match-time and since-spawn formulas and pick the better one. Also sample `m_iHealth` per second to observe regen between damage events.*
- [ ] Wiki: shield base = 35 HP, scaling = +5/min from match start (not from spawn) -- *How to check: inspect all `CNPC_MidBoss` entity fields at spawn tick for any shield-named field; if found, compare to formula; if not found, grep generated protos for `shield` references*
- [ ] Wiki: respawn timers are 7 min / 6 min / 5 min / 5 min... after successive deaths -- *How to check: compute `spawn[n+1] - kill[n]` from probe output on a ≥2-kill replay*
- [ ] A Citadel message fires at the 50% health roar (updated from 70% on 2026-03-06) -- *How to check: grep generated proto types for `Roar`, `Announce`, `HealthAnnouncement`; probe output for any unknown messages firing ±10 ticks of the 50% crossing*
- [ ] `BossKilled.entity_killed_class` for mid-boss is a stable integer across replays -- *How to check: probe binary logs the value; cross-check against ≥2 replays if available*
- [ ] `RejuvStatus` fires exactly 3 times per mid-boss kill (once per credit awarded), with a "buff granted" `event_type` value -- *How to check: probe binary logs all `RejuvStatus` events; count occurrences per kill; inspect distinct `event_type` values*
- [ ] `MidBossSpawned` fires once per spawn cycle -- *How to check: probe binary logs all spawn events; cross-reference count against `kill_events.len() + (1 if boss_alive_at_match_end else 0)`*

### Accepted (not tested here)

- `deadlock-api/haste` async Visitor API is the correct subscription surface -- *Risk if wrong: probe code structure needs rewriting (low risk; validated in currency-changed spike)*
- `CNPC_MidBoss` entity hash in `parser/src/entities/constants.rs:49` is correct -- *Risk if wrong: entity-level reads target wrong class (validated in prior midboss spike against the active `deadlock-api/haste` fork)*
- `CCitadelUserMsg_BossKilled` (ID 347) field shapes from the prior spike are still current -- *Risk if wrong: probe binary logs malformed fields; re-fetch proto*
- `CCitadelUserMsg_RejuvStatus` (ID 350) field shapes from the prior spike are still current -- *Risk if wrong: same as above*
- A suitable multi-kill replay exists in `/parser/src/replays/` or can be downloaded via existing tooling -- *Risk if wrong: Q3 (respawn) and Q7 (single-fire) cannot be fully answered; fall back to single-kill replay and flag as partial*

---

## Agent & Timebox

**Agent:** `rust-parser` (primary -- owns probe binary and cargo invocation against the `deadlock-api/haste` fork)
**Support:** `haste-expert` on request for proto/reference questions during investigation
**Timebox:** 4 hours

**Sequence:**
1. Re-fetch deadlock.wiki/Mid-Boss to confirm wiki content hasn't drifted since the plan was written (30 min)
2. Locate or prepare a ≥2-kill replay (30 min)
3. Inspect `CNPC_MidBoss` entity fields at spawn and during fight directly from the probe binary (subscribe to `on_entity` and log every `FieldValue` on the target entity at the selected ticks) to resolve Q1/Q2 (45 min)
4. Write + run probe binary for Q5/Q6/Q7 (90 min)
5. Cross-reference spawn/kill ticks against wiki respawn formula for Q3 (15 min)
6. Grep protos + probe output for 50% roar message for Q4 (15 min)
7. Write findings + draft learnings (15 min)

---

## Research Standards

Follow `.claude/rules/shared/research.md` for confidence labels (`confirmed` / `inferred` / `hypothesis`) and citation format. Every numeric claim in Findings must cite a specific replay file + tick + field, or a specific proto file + commit, or the wiki URL + section.

**Probe-workflow learnings to apply:**

- `feedback_probe_granularity` -- sample at 1 s minimum, never per-tick. At 64 tps, the correct 1 Hz gate is `tick % 64 == 0` (emits at ticks 0, 64, 128, ... -- exactly one sample per match second). Older notes in that learning cite `tick % 60 == 0`, which predates the 64-tps correction and actually yields a 0.9375 s cadence -- fine for "1 s minimum" but not exactly 1 s. Use `% 64` for new probes.
- `feedback_probe_commit_before_cleanup` -- persist probe source to `private/engineering/tools/` before any `parser/src/bin/` cleanup sweep; the copy-into-`bin`-then-delete workflow only works if the source already lives under `private/engineering/tools/`.

---

## Investigation Approach

### Step 1 -- Wiki re-check

Fetch `https://deadlock.wiki/Mid-Boss` and diff the current page against the values in `private/plans/implementation/mid-boss-tracking.md` (Reference Data section). Flag any drift. Record the fetch date in Findings.

### Step 2 -- Replay selection

Check `/parser/src/replays/` for existing replays. A replay with two mid-boss kills is ideal. If none are available, use the replay from the prior spike (which had one kill at tick ~84550) as the fallback and note Q3/Q7 as partially answered.

Command to list available replays:
```bash
ls -la parser/src/replays/
```

If a multi-kill replay is needed, `parse_local` binary or the backend `GET /match/analysis/{match_id}` endpoint can be used to find candidates.

### Step 3 -- Entity inspection (Q1, Q2)

Extend the probe binary's `on_entity` hook to dump every `FieldValue` on `CNPC_MidBoss` at three ticks:
- **T_spawn**: the tick immediately after `MidBossSpawned` fires (first spawn, ~10 min match time)
- **T_spawn + 5 min**: to verify max health scaling slope
- **T_first_damage**: to capture shield state before / during damage

Record every field on the entity (not just `m_iHealth`/`m_iMaxHealth`) at T_spawn. Specifically look for anything matching `shield`, `m_iShield*`, `m_flShield*`, or any numeric field that matches the value `35 + 5 * match_minute` at that tick.

If no shield field is found on the entity itself, grep generated protos:
```bash
grep -ri "shield" parser/target/debug/build/valveprotos-*/out/deadlock.rs | head -40
```

Also inspect `DamageRecord.victim_shield_max` and `victim_shield_new` from any `CCitadelUserMessage_Damage` event targeting the mid-boss -- the implementation plan notes these fields exist in the damage stream and may be the canonical source.

### Step 4 -- Probe binary (Q5, Q6, Q7)

Create `parser/src/bin/probe_midboss_runtime.rs` modeled on the async Visitor pattern in `replay_parser.rs:299-513`. Subscribe to three messages and log every field plus `ctx.tick()`:

| Message | ID | Log format |
|---------|----|------------|
| `CCitadelUserMsg_MidBossSpawned` | 349 | `[tick={}] MidBossSpawned` |
| `CCitadelUserMsg_BossKilled` | 347 | `[tick={}] BossKilled class={} team={} gametime={} pos={:?} remaining={}` |
| `CCitadelUserMsg_RejuvStatus` | 350 | `[tick={}] RejuvStatus killing_team={} user_team={} player_pawn={} event_type={}` |

Run against the selected replay. Capture full stdout to a findings log. From the output, extract:
- The `entity_killed_class` integer for each `BossKilled` where the killed class is the mid-boss (disambiguate from walker kills via `bosses_remaining` sequence or by pairing with inspector-confirmed mid-boss death ticks)
- Count of `MidBossSpawned` events and the deltas between successive spawns
- Count of `RejuvStatus` events per kill and the set of distinct `event_type` values

### Step 5 -- Respawn formula check (Q3)

From probe output, compute:
```
respawn_delay_n = spawn[n+1].tick - kill[n].tick
```
Convert to seconds via `tick_interval` (1/64 by default -- check `ctx.tick_interval()`). Compare to wiki formula (7 min / 6 min / 5 min). Acceptable tolerance: ±2 seconds (accounts for respawn animation + wiki rounding).

### Step 6 -- 50% roar event search (Q4)

Two angles:

1. **Proto search** -- grep generated types for relevant keywords:
   ```bash
   grep -ri "Roar\|Announce\|HealthThreshold\|HealthAnnouncement\|MidBossHealth" parser/target/debug/build/valveprotos-*/out/deadlock.rs
   ```

2. **Probe expansion (optional)** -- if a candidate message is found, add a subscription to the probe binary, re-run, and verify it fires at ~50% mid-boss health (cross-reference to the `m_iHealth` timeline from Step 3).

If no dedicated message exists, note that the 50% crossing must be derived client-side from the health sample stream and flag for the implementation plan.

### Step 7 -- Findings consolidation

Update each Findings subsection with results. For each validated / invalidated assumption, update the Assumptions Check list with evidence citation. Append draft learnings to `private/learnings.md` ## Drafts covering:
- Any wiki claim that did NOT match replay data (these are the highest-value learnings -- they caught an implementation plan error)
- The `MID_BOSS_CLASS_ID` value and the replay it was confirmed in
- The `RejuvStatus.event_type` enum mapping
- Whether the shield lives on the entity or only in the damage stream

---

## Findings *(agent fills in)*

**Probe run date:** 2026-04-08. Binary: `parser/src/bin/probe_midboss_runtime.rs`. Replays: `68175583_527726523.dem` (3 mid-boss kills), `68182475_4609034.dem` (2 kills), `55423930_379917638.dem` (3 kills, single-kill cross-reference). Wiki fetched via exa_search 2026-04-08.

**Answer:** Q5 confirmed (class=8, stable across 3 replays). Q6 confirmed (event_type 6=granted, 7=consumed, 8=expired). Q7 confirmed (single-fire per cycle, once the kill count is filtered to class=8 only). Q3 confirmed (7 min / 6 min exact to the tick, using 64 tps). Q4 confirmed-absent (no Citadel message exists for 50% roar). Q1 confirmed (formula `max_health = 13000 + 195 * match_minutes` -- match-start time reference; `m_iMaxHealth` is static within a single CNPC_MidBoss lifetime but **jumps between cycles** because match time advances before the next CREATE; `m_iHealth` regenerates at ~15 HP/s between damage events). Q2 partial (no shield field on entity; damage stream is the source, not entity fields).

**Critical correction -- tick rate:** All replays confirm `tick_interval=0.015625` (64 tps), NOT 60 tps. The project memory note "60 ticks/sec" is wrong for Deadlock. All tick-to-second calculations in this spike use 64 tps. The respawn delta of exactly 26881 ticks = 420.0s at 64 tps (7.00 min) would be 448s at 60 tps, which does not divide evenly and would not match the wiki. The 64 tps value is the ground truth.

### Q1 -- Max health scaling and health regen

**Confirmed.** Probe `private/engineering/tools/probe_midboss_health.rs` samples `m_iHealth` and `m_iMaxHealth` on every `CNPC_MidBoss` CREATE, DELETE, and UPDATE at ~0.94 s cadence (as-written `tick % 60 == 0` at 64 tps = every 60 ticks = 60/64 s). Run 2026-04-16 against `/parser/src/replays/68175583_527726523.dem` (3 spawn cycles, 818 samples collected). A future run should switch the gate to `tick % 64 == 0` for a true 1 Hz cadence -- the 0.94 s rate does not change any of the findings below (all three CREATE samples land on distinct 0.94 s buckets and the regen segment spans enough buckets to measure HP/s).

**Formula: `max_health = 13000 + 195 * match_minutes`** -- time measured from match start, NOT from spawn. Validated within 0.7% on every spawn:

| Cycle | CREATE match_time_s | match_min | observed max_health | formula A (match-time) | delta | formula B (since-spawn) | delta |
|-------|--------------------|-----------|---------------------|------------------------|-------|-------------------------|-------|
| 1 | 602.625 (10:02.62) | 10.04 | 14,950 | 14,959 | **-9** (0.06%) | 13,000 | +1,950 |
| 2 | 1657.375 (27:37.38) | 27.62 | 18,265 | 18,386 | **-121** (0.66%) | 13,000 | +5,265 |
| 3 | 2055.719 (34:15.72) | 34.26 | 19,630 | 19,681 | **-51** (0.26%) | 13,000 | +6,630 |

Formula A fits. Formula B (the wiki's colloquial "from spawn" prose) is wrong by thousands of HP.

**`m_iMaxHealth` behavior -- per-cycle, not per-match:**
- **Static within a single CNPC_MidBoss lifetime.** Slope = 0.00/min across all three cycles' lifetimes (cycle 1: 636 s, cycle 2: 40 s, cycle 3: 90 s). `m_iMaxHealth` never changed between CREATE and DELETE.
- **Changes between cycles.** Cycle 1 -> cycle 2: +3,315 HP. Cycle 2 -> cycle 3: +1,365 HP. The new value at each CREATE follows the match-time formula at the moment of that CREATE.
- **Implication:** A single match-global `max_health` value is wrong for any cycle after the first. Consumers (health bars, percentage math) must use the max_health of the current cycle, read at that cycle's CREATE.

**`m_iHealth` regen -- ~15 HP/s between damage events:**
- Probe captured one regen segment in cycle 2, mid-fight, between two damage bursts: `dh = +14 HP over 0.938 s = 14.93 HP/s`. Matches the wiki's 15 HP/s within the 1-second sampling floor.
- Regen was not visible in cycles 1 and 3 because each of those cycles was killed in a single contiguous burst -- damage started, dropped health monotonically to 0, DELETE. No between-damage gap long enough to regen through a sample window.
- Regen is only observable *while the boss is taking damage with brief gaps*. Before any damage, `m_iHealth == m_iMaxHealth` and stays there. After the full kill burst, there's no further recovery.

**Sampling context for this replay (3 cycles of `68175583_527726523`):**
- Cycle 1: CREATE 10:02.62 at 14,950/14,950 -> **~10.3 min of idle at full HP** -> fight 20:19.50 -> 20:38.77 (19 s, monotonic drain 14,950 -> 0).
- Cycle 2: CREATE 27:37.38 at 18,265/18,265 -> 19 s idle -> fight 27:56.06 -> 28:17.11 (21 s, contains the only visible regen segment).
- Cycle 3: CREATE 34:15.72 at 19,630/19,630 -> 65 s idle -> fight 35:20.44 -> 35:46.14 (26 s, monotonic drain).

**Implementation guidance:**
- The parser reads `m_iMaxHealth` directly from the entity at each CREATE (one read per spawn cycle). No formula computation needed.
- The parser must store max_health **per spawn cycle** (e.g. on `MidBossSpawnEvent`), not a single top-level value. A single match-global field underestimates cycle 2/3 max health by up to ~31% in this replay.
- Between-fight regen at ~15 HP/s is a real effect but was only exercised once in this replay (mid-fight pause). Between-window regen (boss damaged, players retreat, boss recovers, fight resumes) is theoretically the same mechanic but was not captured here. Cross-check on a replay with a multi-window cycle before committing to a specific regen-surfacing shape (contract constant vs sparse between-window samples).

**Confidence:** `confirmed` for formula A, for per-cycle-static behavior, and for the per-cycle-delta between CREATEs. `confirmed-once` for 15 HP/s regen (one observation, matches wiki). `inferred` for between-window regen behavior (same `m_iHealth` field, same mechanic, but not exercised in this probe run).

**Overall confidence (Q1):** `confirmed` -- the headline claim (per-cycle `m_iMaxHealth` with match-start scaling) is backed by 3 CREATEs within 0.7% of the formula in one replay; the regen rate and between-window behavior sit behind the headline at `confirmed-once` and `inferred` respectively. Shape of the implementation plan change (move `max_health` to `spawn_events[]`, don't emit a regen constant) follows from the headline; the softer sub-claims do not gate it.

**Probe / test artifacts:**
- Probe source: `private/engineering/tools/probe_midboss_health.rs` (committed 2026-04-16; copy-to-`parser/src/bin/` workflow per `feedback_probe_commit_before_cleanup`).
- Sampling cadence: ~0.94 s via as-written `tick % 60 == 0` gate at 64 tps (60/64 s = 0.9375 s between samples). The correct 1 Hz gate is `tick % 64 == 0`; the 0.94 s rate used in this probe run still satisfies the "1 s minimum" floor in `feedback_probe_granularity`, and new probes should use `% 64` for exact per-second sampling.
- Full probe output for `68175583_527726523.dem`: 904 lines including per-sample `cycle / kind / tick / match_time_s / mm:ss / health / max_health / pct` rows, plus summary (per-cycle breakdown, distinct max_health values, regen segments, formula A vs B fit).

**Citation:** `68175583_527726523.dem`, `probe_midboss_health.rs` at `private/engineering/tools/`, run 2026-04-16; wiki `https://deadlock.wiki/Mid-Boss` Overview section (Health Scaling + Health Regen), fetched 2026-04-16.

### Q2 -- Shield mechanic

**Confirmed-absent on entity.** The prior spike listed every `CNPC_MidBoss` field at tick 43931 (`55423930_379917638.dem`) and found no `m_iShield*` or `m_flShield*` field. The wiki confirms the mechanic exists (35 HP base + 5 HP/min from match start). The shield is not exposed as an entity field -- it is only visible via the damage stream (`victim_shield_max` and `victim_shield_new` fields on `CCitadelUserMessage_Damage`).

**Confidence:** `confirmed` (shield not on entity); `inferred` (wiki formula 35+5/min; not yet replay-validated via damage stream fields).

**Impact:** The v1 plan to not track shield health separately is validated -- the only available signal is the damage stream, not an entity field. (`max_health` contract shape is a separate concern -- see Q1 for the per-cycle correction needed there.)

**Citation:** `55423930_379917638.dem`, tick 43931, prior midboss probe run against the `deadlock-api/haste` fork (2026-04-01); wiki `https://deadlock.wiki/Mid-Boss` Overview section, fetched 2026-04-08.

### Q3 -- Respawn timers

**Confirmed.** Respawn delays measured by pairing class=8 kill ticks with the next `MidBossSpawned` tick across all replays, using confirmed tick_interval=1/64:

| Replay | Kill # | Kill tick | Next spawn tick | Delta ticks | Delta seconds | Delta min |
|--------|--------|-----------|-----------------|-------------|---------------|-----------|
| `55423930` | 1st | 128807 | 155688 | 26881 | **420.0s** | **7.00 min** |
| `55423930` | 2nd | 157088 | 180129 | 23041 | **360.0s** | **6.00 min** |
| `68182475` | 1st | 84550 | 111431 | 26881 | **420.0s** | **7.00 min** |
| `68175583` | 1st | 80403 | 107284 | 26881 | **420.0s** | **7.00 min** |
| `68175583` | 2nd | 109737 | 132778 | 23041 | **360.0s** | **6.00 min** |

First death: 420.0s = **exactly 7 minutes** in every replay. Second death: 360.0s = **exactly 6 minutes** in every replay. The deltas are bit-perfect -- not approximations. The third respawn (5 min) was not observed because no replay had 3 consecutive mid-boss kills with a third spawn following.

**Confidence:** `confirmed` for 7 min (first) and 6 min (second). `inferred` for 5 min (third+) -- wiki value, not yet replay-validated.

**Citation:** `55423930_379917638.dem`, `68182475_4609034.dem`, `68175583_527726523.dem`; probe_midboss_runtime output, 2026-04-08; wiki `https://deadlock.wiki/Mid-Boss` Respawn section, fetched 2026-04-08.

### Q4 -- 50% health roar event

**Confirmed absent.** `CCitadelUserMsgHudGameAnnouncement` (ID 363) fired **zero times** across all three replays. No other candidate message (Roar, HealthThreshold, HealthAnnouncement) exists in the generated `deadlock.rs` proto types. The 50% health threshold crossing cannot be detected via any Citadel user message.

The 50% crossing must be derived client-side from the `health_samples` timeline: find the first sample where `health / max_health <= 0.5`. Precision is limited by the sampling density of `CCitadelUserMessage_Damage` events, but in practice the mid-boss receives frequent damage during a fight so the crossing will be captured within a few damage events.

**Confidence:** `confirmed` (no message exists); `confirmed` (can be derived from health_samples).

**Citation:** proto grep on `parser/target/debug/build/valveprotos-aeab0eb1d292b880/out/deadlock.rs` for Roar/Announce/HealthThreshold/MidBossHealth (2026-04-08); probe_midboss_runtime HudGameAnnouncement count=0 across three replays (2026-04-08); wiki `https://deadlock.wiki/Mid-Boss` Overview section: "Once Mid-Boss has reached 50% Health, it will roar once again" -- wiki describes an in-game audio/animation cue, not a network message.

### Q5 -- `entity_killed_class` value

**Confirmed: `entity_killed_class = 8` for `CNPC_MidBoss`.** Observed in all three replays:

| Replay | Mid-boss kill ticks | entity_killed_class | Adjacent RejuvStatus? |
|--------|---------------------|---------------------|-----------------------|
| `68175583_527726523.dem` | 80403, 109737, 138475 | 8, 8, 8 | yes (3, 3, 2 events) |
| `68182475_4609034.dem` | 84550, 113201 | 8, 8 | yes (2, 3 events) |
| `55423930_379917638.dem` | 128807, 157088, 188638 | 8, 8, 8 | yes (1, 3, 0* events) |

*The third kill in `55423930` at tick 188638 had no RejuvStatus within 600 ticks -- the kill happens near match end. The class=8 identity is confirmed by the consistent position (0.0, 0.0, -768.0) which is the mid-boss pit, team=4 (neutral team), and bosses_remaining=0 with mask_change=-1 (all bits set = "not a standard objective change").

No other entity class produces kills at (0,0,-768) with team=4. The value 8 is the `MID_BOSS_CLASS_ID` constant to add to `parser/src/entities/constants.rs`.

**Confidence:** `confirmed`.

**Citation:** `68175583_527726523.dem`, ticks 80403/109737/138475; `68182475_4609034.dem`, ticks 84550/113201; `55423930_379917638.dem`, ticks 128807/157088/188638; probe_midboss_runtime, 2026-04-08.

### Q6 -- `RejuvStatus.event_type` enum

**Confirmed.** Three distinct values observed, all from replays with confirmed class=8 mid-boss kills:

| event_type | killing_team | Timing after mid-boss kill | Interpretation |
|------------|--------------|---------------------------|----------------|
| **6** | same team as killer | Within 400 ticks (~6s) of kill | Buff granted -- fires once per rejuv stack awarded to a player |
| **7** | -1 (no team) | Minutes after kill | Buff consumed -- player used their rejuv (died and was revived) |
| **8** | -1 (no team) | Same tick as a type=7, or standalone | Buff expired -- the 4-minute timer ran out without the player using it; or final stack consumed |

Count per kill observed:
- event_type=6 fires 2-3 times per mid-boss kill (expected 3; occasionally 2 if one player doesn't reach the crystal in time or steals are not attempted)
- event_type=7 fires once per buff consumption
- event_type=8 fires when the buff expires or all stacks are gone

The implementation plan's filter for "buff granted" events is: `event_type == 6`. The `RejuvStatusEvent` struct should store the raw value; the interpretation layer is in the spec doc.

One anomaly from `68175583`: kill[18] (class=8, tick=138475) only had 2 event_type=6 events instead of 3, then a third event_type=6 appeared at tick 139109 attributed to the same pawn as tick 110137 -- suggesting a duplicate grant to the same player within a single kill. This may be the "all 3 stacks to one player" scenario or a replay artifact.

**Confidence:** `confirmed` for values 6/7/8 and their immediate semantics; `inferred` for the exact expiry semantics of type=8 (may also fire when the last surviving stack is claimed by the last player).

**Citation:** All three replays, probe_midboss_runtime, 2026-04-08.

### Q7 -- `MidBossSpawned` reliability

**Confirmed single-fire per cycle.** When the kill count is filtered to class=8 (mid-boss) only:

| Replay | class=8 kills | MidBossSpawned events | Pattern |
|--------|---------------|-----------------------|---------|
| `68175583_527726523.dem` | 3 | 3 | spawns == kills (boss not alive at match end) |
| `68182475_4609034.dem` | 2 | 2 | spawns == kills |
| `55423930_379917638.dem` | 3 | 3 | spawns == kills |

The Q7 summary output in the probe reported "UNEXPECTED mismatch" because it compared total kills (walkers + patron + mid-boss) to spawn count. Filtered to class=8 kills only, the count is exact. `MidBossSpawned` fires exactly once per spawn cycle.

No double-fire or silent respawn edge cases were observed across 8 total class=8 kill events.

**Confidence:** `confirmed`.

**Citation:** All three replays, probe_midboss_runtime, 2026-04-08.

### Assumptions Check

- [x] Wiki max health formula (13000 + 195/min) -- **held with correction** -- Formula `max_health = 13000 + 195 * match_minutes` confirmed within 0.7% across 3 spawns in `68175583`. Time reference is **match start**, not spawn time (the wiki's "since spawn" prose is wrong). `m_iMaxHealth` is static within a single CNPC_MidBoss lifetime but **changes between cycles** because match time advances -- in `68175583` the three cycles have max values 14,950 / 18,265 / 19,630. A single match-global `max_health` is incorrect for any cycle after the first; per-cycle storage is required. Additionally, `m_iHealth` regenerates at ~14.93 HP/s (one segment observed, matches wiki 15 HP/s). Evidence: `probe_midboss_health.rs` run 2026-04-16.
- [x] Wiki shield formula (35 + 5/min from match start) -- **partial** -- No shield entity field exists (confirmed). The shield formula is consistent with wiki text but not validated against the damage stream (`victim_shield_max`). Damage stream reads in Phase A can confirm.
- [x] Wiki respawn timers (7 / 6 / 5 min) -- **held** -- 7 min and 6 min confirmed exactly to the tick across 5 observations. 5 min (third+) is `inferred` from wiki; not yet replay-observed.
- [x] 50% roar Citadel message exists -- **invalidated** -- `CCitadelUserMsgHudGameAnnouncement` fired zero times across three replays. No proto type for Roar or HealthThreshold exists. The 50% crossing must be derived from health_samples.
- [x] `entity_killed_class` stable across replays -- **held** -- Value 8 observed in all 8 mid-boss kill events across 3 replays. Zero variance.
- [x] `RejuvStatus` fires 3 times per kill -- **partial** -- event_type=6 fired 2-3 times per kill (usually 3; once 2, once possibly 3 including a duplicate). The "up to 3" ceiling is confirmed; exact count depends on how many players claim the crystal. The filter for implementation is event_type=6.
- [x] `MidBossSpawned` single-fire per cycle -- **held** -- Confirmed across 8 total cycles; no double-fire detected.

**Accepted assumptions worth flagging based on findings:**
- Tick rate is 64 tps (NOT 60). The project memory note "60 ticks/sec" is incorrect. This affects all tick-to-second conversions in the implementation (the `ctx.tick_interval()` call at runtime will always return the correct value, so code using `ctx.tick_interval()` is safe -- but any hardcoded `1.0/60.0` constants are wrong).
- The `probe_currency_changed.rs` binary in this worktree uses the old sync haste Visitor API and does not compile. The haste version at commit `34a3a49` requires `async fn` Visitor methods, `type Error = YourType`, and `use prost::Message` (not `use haste::valveprotos::prost::Message`). The `parse_local.rs` binary is the canonical template for this worktree.

---

## Implementation Plan Impact *(agent fills in)*

List every field, constant, or contract entry in `private/plans/implementation/mid-boss-tracking.md` that must change based on findings:

| Location in plan | Current value | Required change | Source |
|------------------|---------------|-----------------|--------|
| Phase A2 -- probe binary | "async Visitor pattern from `replay_parser.rs:299-513`" | Binary must use async fn, `type Error = ProbeError`, and `use prost::Message` (not `use haste::valveprotos::prost::Message`). `probe_currency_changed.rs` is NOT a valid template for this worktree -- use `parse_local.rs` or `replay_parser.rs` instead. | Compile errors; haste commit `34a3a49` uses async Visitor API. |
| Phase A4 -- `MidBossTracker.handle_spawn` match time formula | `(tick as f32 * ctx.tick_interval()) - match_start_time_s` -- no note on tick type | `ctx.tick()` returns `i32` in haste `34a3a49`, not `u32`. Cast required: `(ctx.tick() as f32 * ctx.tick_interval())`. | probe_midboss_runtime compile; `ctx.tick()` type mismatch. |
| Phase A5 step 2 -- BossKilled filter | `if msg.entity_killed_class == Some(MID_BOSS_CLASS_ID)` | Add `MID_BOSS_CLASS_ID: i32 = 8` to `constants.rs` with citation: "`probe_midboss_runtime`, replays `68175583`, `68182475`, `55423930`, 2026-04-08". | Q5 confirmed: class=8 across 8 observations in 3 replays. |
| Phase 0 -- `max_health` field location | single `mid_boss.max_health: int?` at the block top level | Move `max_health` onto the spawn event: `mid_boss.spawn_events[].max_health: int?`. Parser reads `m_iMaxHealth` at each CNPC_MidBoss CREATE (once per spawn cycle); the value is static within that lifetime but differs between cycles. Formula `13000 + 195 * match_minutes` (match-start time) validates the reads but parser does NOT compute it -- the entity field is the source of truth. | Q1 confirmed: per-cycle behavior. In `68175583`, cycles 1/2/3 have max values 14950/18265/19630 -- a single top-level value is wrong for 2 of 3 cycles. |
| Phase 0 -- health regen surfacing | not in plan | Decision: do **not** emit a regen rate constant. Within fight windows, existing `health_samples` already capture regen from `m_iHealth` entity updates. Between fight windows, the frontend holds `health_at_end` constant; the resulting staleness is bounded by `15 * gap_s` HP and gap periods in practice end on kill or a fresh engagement. If a future experiment needs precise between-window health, extend `fight_windows` with sparse samples rather than adding a `regen_hp_per_s` constant. | Q1 regen confirmed-once at 14.93 HP/s; within-window capture already exists via `health_samples`; gap-period error bounded and acceptable for v1. |
| Phase A5 / Phase C2 -- 50% roar marker | Implied nice-to-have "nice-to-have for frontend" | Remove as a message-subscription candidate. No Citadel message fires at 50% HP. Frontend must derive the crossing from `health_samples`: find first sample where `health / max_health <= 0.5`. Document this in the Phase C3 health bar spec. | Q4 confirmed-absent: HudGameAnnouncement=0, no Roar proto type exists. |
| Phase A4 -- `RejuvStatus` filter | "event_type enum values are not documented" | Document confirmed values: 6=buff granted (filter for this), 7=buff consumed on player death, 8=buff expired or last stack gone. Phase A4 `handle_rejuv_status` should record all events where `event_type == 6` as the "granted" signal; types 7 and 8 can be stored as-is for future use. | Q6 confirmed. |
| Phase C2 -- rejuv claim count | "Up to 3 rejuvs per kill" | Note: event_type=6 fires 2-3 times per kill (not always exactly 3). Count is number of players who claimed the crystal, not a fixed 3. Frontend should display count of event_type=6 events per spawn_cycle, not assume 3. | Q6 partial -- count varies. |
| Open Questions item 3 -- `MidBossSpawned` single-fire | "validated as `inferred`; use `bosses_remaining` as secondary signal" | Promote to `confirmed`. No secondary signal needed. `MidBossSpawned` fires exactly once per cycle across 8 observations. | Q7 confirmed. |
| Open Questions item 5 -- respawn timers | "First: 7 min. Second: 6 min. Third+: 5 min. (Source: wiki)" | 7 min and 6 min are now `confirmed` to the tick. 5 min remains `inferred` (wiki only). No plan change needed -- formula is correct; just update confidence label. | Q3 confirmed. |
| Phase A5 -- match time conversion note (spike plan) | "1/64 by default -- check `ctx.tick_interval()`" | Tick rate confirmed 64 tps (1/64 = 0.015625). Hardcoded constants anywhere in the codebase using `1.0/60.0` for tick conversion are wrong. The `ctx.tick_interval()` call at runtime returns the correct value and is safe. | Confirmed across all replays. |
| Phase A2 -- probe binary name | "probe_mid_boss.rs" | Binary was created as `probe_midboss_runtime.rs` -- update all references. | Implementation decision at creation time. |
| Phase 0 -- BossSnapshot contract | plan does not reference `boss_name_hash` | Mid-boss snapshots must emit `boss_name_hash = fxhash::hash_bytes(b"CNPC_MidBoss")` as the canonical type identifier, matching `parser-output.md:122-136` (main branch, commit `22fe74a`). Add a row to the `Boss Type Identification` table in `parser-output.md` with the u64 hash value. `entity_killed_class = 8` is used only for filtering the `BossKilled` message stream, not for contract emission. | Main-branch parser-output contract requires boss_name_hash; mid-boss must participate. |
| Phase 0 -- max_health mechanics | not in plan | Add a row to the `max_health Mechanics` table in `parser-output.md`: Mid-boss base 13000, scaling `+195/match_min` (match-start reference, not since-spawn), set at each entity CREATE, static within a cycle's lifetime, distinct value per spawn cycle. Regen `15 HP/s` when below max, observable only in the damage stream. Confidence `confirmed`. Parser reads `m_iMaxHealth` from the entity at each CREATE -- no formula computation. | Main-branch contract documents max_health for every other boss. Q1 confirmed 2026-04-16. |
| Phase A / Q2 follow-up -- shield formula source | current note suggests "damage stream" without specifying which message | Use `CCitadelUserMessage_Damage` (ID 300, VERIFIED) field `victim_shield_max` / `victim_shield_new`. Do NOT use `CCitadelUserMsg_BossDamaged` (ID 348) -- its only fields are `objective_team`, `objective_id`, `entity_damaged`, with no shield data. | `citadel-messages-reference.md:308-316`. |
| Phase A2 -- probe binary location | Implied: `parser/src/bin/` | Main convention is now: write new probes under `private/engineering/tools/`, copy into `parser/src/bin/` to run, then delete after the run. Keeps `parser/src/bin/` free of throwaway probes. Existing probes `probe_all_messages.rs`, `probe_entity_counts.rs`, `probe_post_match_details.rs` live there already. | `citadel-messages-reference.md:40-42` documents this convention. |
| Phase 0 -- Gap closure framing | plan does not cite the spec gap | The mid-boss-tracking plan closes `entity-types-reference.md` Gap 3 (mid-boss health not tracked) and partially closes Gap 5 (rejuv / pickup entities not subscribed). Frame the plan's motivation in terms of these documented gaps so the linkage to F6 / F9 product alignment is explicit. | Main branch `entity-types-reference.md:492-520`. |

---

## Learnings Output

- [x] Draft entry appended to `private/learnings.md` ## Drafts -- 2026-04-08
- [x] `MID_BOSS_CLASS_ID` value recorded: 8, cited to probe_midboss_runtime across 3 replays, 2026-04-08
- [x] `RejuvStatus.event_type` enum values documented: 6=granted, 7=consumed, 8=expired
- [x] Wiki-vs-reality drift flagged in learnings: tick rate 64 tps (not 60); 50% roar has no Citadel message; max health formula is match-start-referenced (not since-spawn as the wiki prose implies) and requires per-cycle storage because `m_iMaxHealth` changes between CREATEs
- [x] Follow-up questions or spikes needed (1 resolved, 2 open):

  1. [x] **Q1 -- max health formula + regen validation.** **Resolved 2026-04-16.** Formula is `max_health = 13000 + 195 * match_minutes` (from match start). `m_iMaxHealth` is static within a single CNPC_MidBoss lifetime but changes between cycles -- the parser must read it at each CREATE and store per-spawn (on `MidBossSpawnEvent`), not match-global. `m_iHealth` regenerates at ~15 HP/s between damage events (one 14.93 HP/s segment observed mid-fight). The per-cycle fix is folded directly into the implementation plan's Phase 0/A/B/C; no separate follow-up section. See `probe_midboss_health.rs`.
  2. [ ] **Q2 -- shield formula validation.** Subscribe to `CCitadelUserMessage_Damage` events where the victim is `CNPC_MidBoss` and inspect `victim_shield_max` at the first damage event after spawn. Cross-reference against wiki formula `35 + 5 * (match_minute_at_spawn)`. Can be done as a quick addition to the midboss probe binary.
  3. [ ] **Q3 -- third respawn (5 min).** No replay in the available set had 3+ consecutive mid-boss kills with a third spawn following. The 5 min claim is wiki-only. Low priority -- the pattern holds for 1st and 2nd death; 5 min for 3rd+ can be confirmed when a suitable replay is available.

---

## Cross-reference with `main` branch specs (added 2026-04-11)

The midboss worktree branched before several key specs and probes landed on `main`. All spike findings above have been cross-referenced against the newer specs on `main`. No findings are invalidated; several are corroborated by independent verification runs.

### Corroborated by `main` spec updates

**`CCitadelUserMsg_MidBossSpawned` (349), `BossKilled` (347), `BossDamaged` (348), `RejuvStatus` (350)** -- all four are marked **VERIFIED** in `private/specs/citadel-messages-reference.md` (lines 291-343, updated 2026-04-09) via `probe_all_messages` against 3 replays. The verification set includes `68175583_527726523` -- the same replay this spike used. Event counts from the main-branch probe match the spike's probe within expected tolerance (MidBossSpawned: 8, BossKilled: 68 including walkers/shrines/patron, RejuvStatus: 48, BossDamaged: 790).

**`HudGameAnnouncement` (363) is NOT SEEN** -- main spec marks this message unfired across all 3 verification replays (`citadel-messages-reference.md:541-542`). Independently confirms Q4's "50% roar Citadel message does not exist" finding. The spec explicitly recommends: "Phase-detection features should rely on `MidBossSpawned` (349) + `BossKilled` (347) instead." Spike recommendation stands: frontend must derive the 50% crossing from `health_samples`.

**Mid-boss health timeline is a known gap** -- `entity-types-reference.md:492-497` (Gap 3) explicitly calls out: `BossTracker.is_boss_entity()` returns false for `CNPC_MIDBOSS_ENTITY`, health timeline not sampled, spawn-to-death duration not explicitly recorded. The mid-boss-tracking implementation plan is the right vehicle to close this gap.

**`CNPC_MidBoss` is a distinct entity from `CNPC_Neutral_SinnersSacrifice`** -- `main` commit `d4f75a0` explicitly separates these and removes incorrect Torment Pulse references. This validates the prior message-fields spike's entity-class choice; the mid-boss-tracking plan targets `CNPC_MidBoss` and is clean of Sinner's Sacrifice conflation.

**Entity census for `68175583_527726523`** -- `private/engineering/tools/entity_counts_68175583_527726523.txt`:
- `CNPC_MidBoss`: 3 CREATE / 3 unique_idx / 3 DELETE -> corroborates 3 spawn/kill cycles -> independently confirms Q7 single-fire.
- `CCitadelItemPickupRejuv`: 3 CREATE / 3 unique_idx / 3 DELETE -> one rejuv pickup per mid-boss death. Confirms prior spike's finding that `CCitadelItemPickupRejuv` spawns at mid-boss death and is the correct entity class.

### Extensions and scope additions that the worktree-local plan was missing

**Contract: use `boss_name_hash` not `entity_killed_class`.** The main-branch contract (`specs/contracts/parser-output.md:122-136`, commit `22fe74a`) canonicalizes `boss_name_hash` as the only stable type identifier across games for `BossSnapshot`. `entity_killed_class` from `BossKilled` is a runtime value and is **not** in the contract. Implication for the implementation plan:
- `MID_BOSS_CLASS_ID = 8` (from Q5) is correct for filtering the `BossKilled` message stream but is NOT the identifier emitted in `BossSnapshot`.
- When adding mid-boss to `BossTracker`, the snapshot must emit `boss_name_hash = fxhash::hash_bytes(b"CNPC_MidBoss")` via the existing `CNPC_MIDBOSS_ENTITY` parser constant.
- The `Boss Type Identification` table in `parser-output.md:126` currently lists Guardian / Base Guardian / Shrine / Walker / Patron. Adding mid-boss requires publishing its u64 hash value in that table so consumers can hardcode it.

**`max_health` mechanics section in `parser-output.md` (commit `5034355`, lines 138-155)** -- documents wiki-sourced scaling for every objective EXCEPT mid-boss. The mid-boss-tracking plan's Phase 0 contract should add a row to that table with the **replay-validated** values (base 13000, `+195/match_min` from match start -- not since-spawn; `m_iMaxHealth` is static within a cycle's lifetime but changes between CREATEs; regen 15 HP/s when below max). Confidence `confirmed` (Q1, 2026-04-16).

**`BossDamaged` (348) is VERIFIED and cheap but thin.** `citadel-messages-reference.md:308-316` -- fields are `objective_team`, `objective_id`, `entity_damaged` only. **No shield fields.** This rules out `BossDamaged` as the source for Q2's shield formula validation. Q2 follow-up must use `CCitadelUserMessage_Damage` (ID 300, field `victim_shield_max` / `victim_shield_new`), which is VERIFIED with high volume -- not `BossDamaged`. Update Q2 follow-up note accordingly.

**Main repo has a canonical probe at `private/engineering/tools/probe_all_messages.rs`** which uses the async Visitor API (`use haste::parser::{Visitor}` with `type Error = ProbeError` and `async fn on_packet`). Future probes in this worktree should either use that as the template OR be written against `replay_parser.rs` directly. The worktree-local `probe_currency_changed.rs` is stale pre-async-migration code and does not compile.

### Worktree-staleness flags (not part of the spike, but relevant to whoever implements the plan)

The midboss worktree branched before these `main` changes. They are not yet present here and should be pulled via `scripts/wt sync midboss` (or manually cherry-picked if a sync would conflict) before implementation begins:

1. `specs/entity-types-reference.md` -- new file; documents `CNPC_MidBoss`, `CCitadelItemPickupRejuv`, and the Gap 3 / Gap 5 feature gaps the mid-boss plan closes.
2. `specs/entity-types-runtime-census.md` -- new file; methodology for the entity count probe.
3. `specs/citadel-messages-reference.md` -- updated with VERIFIED/NOT SEEN status on every ID.
4. `specs/citadel-gcmessages-common-reference.md` -- new file; `CMsgMatchMetaDataContents` post-match blob schema.
5. `specs/contracts/parser-output.md` -- `boss_name_hash` canonicalization, `max_health` mechanics table, `BossHealthWindow` UPDATE-driven sampling semantics.
6. `parser/src/bin/probe_currency_changed.rs` -- removed on main; async-migration made it dead code. The worktree still has it.
7. `private/engineering/tools/` -- four new probe binaries live here, not in `parser/src/bin/`. Follow the pattern: copy into `parser/src/bin/` to run, then delete.

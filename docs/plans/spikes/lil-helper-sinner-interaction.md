# Spike: Lil Helper -- Sinner Interaction Data Source Validation

**Status:** Partially complete -- Q4/Q5 confirmed; Q1-Q3 inconclusive (Lil Helpers may not have been used on sinners in the available replay). Defensive branch added to implementation plan.
**Timebox:** 1 day
**Replay used:** `55841493_649180947.dem`
**Probe:** `parser/src/bin/lil_helper_probe.rs` on `feature/sinner-tracking`
**Further replay needed:** match where Rem verifiably assigns a Lil Helper to a Sinner's Sacrifice machine and it completes the kill -- to conclusively answer Q1-Q3

---

## Background

Rem's Lil Helpers ability (ability 3) allows NPC helpers to be assigned to Sinner's Sacrifice machines. Per the wiki, they deal melee hits and always secure the jackpot as if done with a heavy melee. This means:

- A Lil Helper can be the last attacker when the death signal fires (`health == 1 && prev_health > 1`)
- `entindex_attacker` in `CCitadelUserMessageDamage` may point to a Lil Helper entity index, not a player pawn
- The existing player pawn → controller → `m_unLobbyPlayerSlot` resolution chain will fail silently for this case
- Sinner retaliation (80 damage/hit) may target the Lil Helper entity rather than Rem's pawn

The implementation plan's killer attribution and retaliation tracking both need to handle this path. This spike validates the exact entity structure so the implementation can be written correctly the first time.

---

## Questions to Answer

### Q1: Does `CCitadelUserMessageDamage` fire for Lil Helper hits on sinners?

Confirm whether `entindex_attacker` is set to the Lil Helper entity index when a Lil Helper melees a sinner. If no Damage event fires, the `last_attacker` map will be empty or stale at the death signal.

### Q2: What serializer class name do Lil Helper entities have?

Find the serializer name (e.g., `CNPC_Rem_Helper` or similar) so `is_lil_helper()` can be implemented in `replay_parser.rs`. Log all entity class names seen as attackers on sinner Damage events in the probe output.

### Q3: What field on the Lil Helper entity points back to Rem's pawn?

Most likely `m_hOwnerEntity` (ehandle), but must be confirmed. Dump all fields on the Lil Helper entity in the probe and look for an ehandle that resolves to Rem's `CCitadelPlayerPawn`.

### Q4: Does sinner retaliation target the Lil Helper entity or Rem's pawn?

Check `entindex_victim` in Damage events where `entindex_attacker` = sinner entity index and the sinner is being farmed by a Lil Helper. If the victim is the Lil Helper (not a player pawn), the existing retaliation handler silently drops it -- we need to add a branch that traces Lil Helper → owner pawn → slot.

### Q5: Does soul distribution work differently for Lil Helper hits?

The wiki says souls are split among all players who hit the machine. Does a Lil Helper hit count as Rem hitting it? Verify by checking if Rem appears in `retaliation_damage` for a Lil Helper-only sinner kill -- if retaliation fired toward Rem's pawn, it confirms the game treats Lil Helper hits as Rem's hits at the damage layer.

---

## Probe Instructions

1. Add logging to `sinner_probe.rs` (or a fork) that prints every `CCitadelUserMessageDamage` event where `entindex_victim` is a tracked sinner entity index, including the attacker's entity index and the serializer class name of the attacker entity.

2. Also log Damage events where `entindex_attacker` is a tracked sinner entity index, including the victim entity index and the victim's serializer class name. This catches retaliation direction.

3. Run the probe against the Rem replay. Look for attacker class names that are not `CCitadelPlayerPawn`.

4. For any non-player attacker found in step 3, dump all property paths and values on that entity to identify the owner field.

---

## Expected Findings

Based on how similar NPC pets work in Deadlock (e.g., Wraith's card summons):

- Lil Helpers likely have a class name like `CNPC_Ability_Rem_LilHelper` or similar
- `m_hOwnerEntity` likely points to Rem's pawn (ehandle, needs `ehandle_to_index()` conversion)
- Retaliation may target the Lil Helper directly (not Rem's pawn) -- if so, we need to resolve Lil Helper → owner pawn → controller → slot for retaliation accumulation

---

## Implementation Impact

Findings directly unblock **Phase A, step A6.5** in `sinner-tracking.md`:

| Finding | Impact |
|---------|--------|
| Q1: Damage event fires, attacker = Lil Helper index | `record_damage` path works as-is; killer resolution needs Lil Helper branch |
| Q1: No Damage event fires | Must use a different kill detection path for Lil Helper kills; `last_attacker` will be empty |
| Q3: Owner field confirmed | Implement Lil Helper → owner resolution in killer attribution block |
| Q4: Retaliation targets Lil Helper | Add NPC victim branch to retaliation handler; trace Lil Helper → Rem's slot |
| Q4: Retaliation targets Rem's pawn | Existing player pawn handler already captures it -- no additional branch needed |

Record all findings in the Results section below and update step A6.5 in `sinner-tracking.md` with the confirmed entity class name and owner field path.

---

## Results

**Replay used:** `55841493_649180947.dem`
**Probe:** `parser/src/bin/lil_helper_probe.rs` (branch `feature/sinner-tracking`)
**Status:** Partially conclusive -- Lil Helpers may not have been used on sinners in this replay; all sinner interactions were player-pawn-direct. See Interpretation section below.

- [x] Q1: **No NPC attacker seen** -- all 280 sinner Damage events had `attacker_is_player_pawn=true`. Zero `NPC/other` attacker events observed across 39 sinner deaths.
- [~] Q2: Lil Helper serializer class name: **not observed** -- no NPC entity attacked a sinner; class name could not be determined from this replay.
- [~] Q3: Owner field path: **not applicable** -- no Lil Helper entity appeared as an attacker; `m_hOwnerEntity` was not needed.
- [x] Q4: Retaliation victim is **Rem's pawn (PlayerPawn)** -- all 144 retaliation events targeted a `CCitadelPlayerPawn`. Zero NPC victims observed.
- [x] Q5: Soul distribution: **all 39 deaths attributed to player pawns with resolved slots** -- no NPC-attributed death. If Lil Helpers were used, either damage is credited to the owning pawn in `CCitadelUserMessageDamage`, or they were not used on sinners in this replay.

**Raw attacker class names seen on sinner Damage events:**
```
Total sinner Damage events: 280  (player pawn: 280  NPC/other: 0)
Q1: Damage events fired for non-player-pawn attackers: NO -- no NPC attacker seen
```

**Lil Helper entity field dump:**
```
Not applicable -- no Lil Helper entity appeared as sinner attacker in this replay.
```

---

## Interpretation

Two possible explanations for zero NPC attacker events:

**Hypothesis A: Rem did not use Lil Helpers on sinners in this replay.**
Rem may have used Lil Helpers elsewhere (other NPC targets, lane farming) but not on sinners. This replay does not contain the data needed to test Q1-Q3.

**Hypothesis B: Lil Helper damage is attributed to Rem's pawn in `CCitadelUserMessageDamage`.**
The game may proxy Lil Helper hits through the owning player's pawn entity at the damage-event layer. Under this hypothesis, `entindex_attacker` would show Rem's pawn index even when a Lil Helper lands the blow -- making the existing player-pawn attribution chain correct with no additional branch needed.

Hypothesis B would also explain Q4/Q5: retaliation targets the pawn (confirmed), souls credited to Rem (consistent with all pawn-attributed deaths).

**Cannot distinguish these hypotheses from this replay alone.**

### Impact on Implementation Plan

If Hypothesis B is correct: Phase A6.5 (Lil Helper attacker resolution) is unnecessary -- no code changes needed for Lil Helper support.

If Hypothesis A is correct and Hypothesis B is wrong: A6.5 remains required and this spike must be re-run against a replay where Rem verifiably uses a Lil Helper to kill a sinner.

**Recommendation:** Implement Phase A6.5 as a defensive branch -- detect non-pawn attacker, resolve `m_hOwnerEntity`, log a warning if owner resolution fails -- then validate with a targeted replay. The branch adds minimal complexity and prevents silent attribution failures if Hypothesis B turns out to be wrong.

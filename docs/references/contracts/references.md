# Contract References

Long-form explanations for fields defined in `parser-output.md` and `backend-api.md`. Tables in those files link here when a note grows past a single sentence.

## MidBossKillEvent: team_killed source

Sourced from `RejuvStatus.killing_team` within the kill's attribution window, **not** `BossKilled.objective_team`. `objective_team` is always `4` (neutral) for `CNPC_MidBoss`, so it conveys no information about which team landed the killing blow. `RejuvStatus.killing_team` is consistent across all grant events for a given kill and is the reliable source.

If the attribution window contains zero rejuv events (degenerate case: e.g. replay ends immediately after the kill), `team_killed` falls back to `0`.

## MidBossKillEvent: team_claimed derivation

Derived by strict majority (`>= 2` of 3) of `event_type == 6` RejuvStatus grants in the attribution window, grouped by `user_team`. Mid-boss kills always produce exactly 3 grants, so one team always reaches the threshold -- `team_claimed` is never null in production data. It is `0` only in the degenerate "no grants observed" fallback.

### Divergence from Valve

`MidBossKillEvent.team_claimed` intentionally differs from `match_metadata.match_info.mid_boss[].team_claimed` on contested cycles. Valve's blob credits the team that *consumed* the buff, collapsing a 2-vs-1 steal into a binary "stolen" verdict. We count *raw grants*, which preserves the contested outcome (e.g. a 2-vs-1 cycle awards `team_claimed` to the team with 2 grants, not the team that happened to consume first).

The Valve value is preserved verbatim in `match_metadata.match_info.mid_boss[].team_claimed` for callers who need to compare against the in-game UI.

## MidBossKillEvent: rejuvs_by_team shape

Raw count of `event_type == 6` RejuvStatus grants in the attribution window, keyed by `user_team` as a string (`"2"` for Amber, `"3"` for Sapphire). Both keys are always present, defaulting to `0`, so downstream code never has to guard for missing keys.

Typical shapes:
- Clean kill: `{"2": 3, "3": 0}` or `{"2": 0, "3": 3}`
- 2-vs-1 steal: `{"2": 2, "3": 1}` or `{"2": 1, "3": 2}`

## MidBossKillEvent: attribution window

For each kill, the parser anchors a 30-second window starting at the matching fight window's `window_end_s` (= last observed damage time on the mid-boss for that spawn cycle). All `RejuvStatusEvent`s within `[anchor, anchor + 30s]` are attributed to that kill.

**Why last-damage instead of `BossKilled.gametime`:** `BossKilled.gametime` lags actual entity death by 7-18 s in observed replays (e.g. cycle 1 of replay `55423930` -- last damage at `1994.6 s`, first grant at `2002.3 s`, `BossKilled.gametime` at `2012.x s`). A window anchored on `matchtime_s` misses every grant; the last-damage anchor sees all three grants consistently.

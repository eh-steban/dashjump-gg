# Coaching Session Event Log Spike

**Status:** Draft
**Timebox:** 1 day

## Context

Anthropic's [Managed Agents](https://www.anthropic.com/engineering/managed-agents) post decouples agent infrastructure into three pieces: **brain** (Claude + harness), **hands** (sandbox/tools), and **session** (append-only event log). The session-as-event-log pattern is the missing foundation for a Deadlock coaching agent -- without it we cannot replay a player's coaching history against a newer model version, and every coaching interaction is disposable. Before committing to build this infrastructure we need one end-to-end proof that the pattern fits our existing parser + backend surface.

---

## Question

Can a single coach↔player conversation about a Deadlock match be modeled as an append-only event log over our existing backend tools, such that the **same log** can be replayed against two different Claude model versions and produce coherent (not necessarily identical) coaching output?

---

## Assumptions

### To Validate

- [ ] An append-only event log of `{user_message, tool_call, tool_result, assistant_message}` is sufficient to fully reconstruct a session -- no hidden harness state required -- *How to check: replay the JSONL in a fresh process with no in-memory carryover and compare final assistant output to the live run*
- [ ] At least three existing backend endpoints (match analysis, lane pressure, player metadata) can be wrapped as MCP-style `execute(name, input) → string` tools without schema translation pain -- *How to check: write the tool adapters; note any fields that need reshaping*
- [ ] Re-running the same event log against a different model (Opus 4.6 → Sonnet 4.6, or vice versa) produces semantically coherent coaching answers -- not token-identical, but addressing the same questions with the same factual grounding -- *How to check: human-read both outputs side by side for a 5-turn scripted conversation*
- [ ] The event log format survives one tool schema change (e.g., add a field to `get_match_analysis` output) without requiring log migration -- *How to check: after the first replay, add a field, replay again, confirm old events still load*

### Accepted (not tested here)

- Player identity, Steam OAuth, and vault-managed credentials are out of scope -- *Risk if wrong: auth model might force session log changes (unlikely -- auth is a harness-entry concern, not a session concern)*
- Sandboxed code execution (pandas over parsed match data) is out of scope -- the spike uses pre-defined tools only -- *Risk if wrong: if coaches need ad-hoc slicing immediately, the tool surface alone may not be enough to prove value; this is a follow-up spike*
- Prompt caching and token cost optimization are out of scope -- *Risk if wrong: production viability may hinge on caching, but viability of the event-log pattern is independent*
- The Anthropic SDK tool-use loop is the right harness foundation -- *Risk if wrong: minor, we can swap harness later -- that's the whole point of the pattern*
- The JSONL event format produced by this spike is throwaway/spike-only and is not a contract -- no update to `private/specs/contracts/backend-api.md` is required; the spike only reads existing endpoints -- *Risk if wrong: if the format gets reused without a formal contract review, schema drift becomes silent*

**Learnings cited in assumptions:**
- `MEMORY.md: feedback_spike_validation.md` -- spike findings must be validated with running code before flowing into implementation plans. The replay test step (running both models against the log in a fresh process) is the required validation gate before any findings here are promoted to an implementation plan.

---

## Agent & Timebox

**Agent:** backend-python
**Timebox:** 1 day (8 hours)

---

## Research Standards

Follow `.claude/rules/research.md` for confidence labels, citation format, and scope discipline. For Anthropic SDK usage patterns (tool use loop, streaming, model IDs), fetch current docs via `context7` or `exa_search --fetch https://docs.anthropic.com/en/api/getting-started` at execution time -- do not assume model IDs from this plan file; resolve them from the live docs.

---

## Investigation Approach

### Setup (1-2 hours)

1. Pick one real match from the backend cache with a completed analysis (any match where `GET /match/analysis/{match_id}` returns cached data -- avoid parser cold starts in the spike)
2. Create a throwaway harness under `private/engineering/tools/coaching_session_spike/` (copy-to-run-then-delete, same convention as existing probes):
   - `tools.py` -- three tool adapters wrapping existing backend endpoints as `execute(name, input) → str`
   - `harness.py` -- minimal tool-use loop using the Anthropic SDK
   - `session.py` -- append-only JSONL writer + reader
   - `replay.py` -- takes a JSONL + model id, replays without live tool execution (tool results come from the log)

### Tool surface (pick exactly 3)

- `get_match_analysis(match_id)` -- cached backend path, returns the full analysis JSON
- `get_lane_pressure(match_id, window_s)` -- if the endpoint exists; otherwise substitute `get_soul_economy`
- `get_player_metadata(steam_id)` -- via deadlock-api proxy

Do **not** add custom analytics. The point is to prove composition over existing surface.

### Scripted conversation (5 turns)

Hand-write a 5-turn conversation a coach would realistically have:
1. "How did player X perform in match Y?"
2. "Where did he lose tempo in the mid game?"
3. "Compare his lane pressure to the winning duo"
4. "What should he work on next match?"
5. "Summarize in 3 bullet points"

Run this once against the most capable model available (resolve the current model ID from Anthropic docs at execution time -- do not hard-code a name), persist every event to JSONL. Record the exact model ID used in the Findings section.

### Replay test

Replay the JSONL against a second, distinct model (resolve ID at execution time). The replay should:
- Load the event log
- Skip tool execution entirely -- feed tool results from the log back to the model as if freshly fetched
- Produce a new assistant message for each turn

Record both conversations in the Findings section and hand-evaluate coherence.

### Schema evolution test

After the replay works, add a synthetic field to one tool's output schema, re-run the replay against the **original** log. Confirm the old events load cleanly (new field absent is OK; crash is not).

---

## Findings

**Answer:** Yes (structural plumbing confirmed; model coherence deferred to live-API spike)

A single coach/player conversation about a Deadlock match CAN be modeled as an append-only JSONL event log such that the same log replays cleanly against a different mock model version with tool results sourced from the log and no in-memory carryover. Whether real Claude models produce semantically coherent coaching answers across versions is not yet confirmed -- that requires a follow-up spike with a live API key.

**Supporting evidence:**
- `private/engineering/tools/coaching_session_spike/runs/live_20260413_164013.jsonl` -- 16 events: 5 user_message, 3 tool_call, 3 tool_result, 5 assistant_message, monotonic idx 0-15
- `runs/live_20260413_164013.replay.claude-sonnet-4-6-mock.jsonl` -- 5 assistant messages with text differing from live on all 5 turns; all tool_results marked `replayed_from_log: true`
- `runs/live_20260413_164013.replay.claude-sonnet-4-6-mock.schema_evo.jsonl` -- schema v2 fixtures (added `rank_badge_level` field) replayed against original log without error; old tool_result strings loaded cleanly
- `harness.py:58-79` -- tool-use loop; tool results execute live adapters and append to log
- `replay.py:66-72` -- tool_result_cursor advances by ordinal; adapters never called

**Overall confidence:** `inferred` for the overall pattern. Structural plumbing is `confirmed`. Real-model coherence is `deferred`.

### Assumptions check

- [x] **Event log is sufficient to fully reconstruct a session** -- **held** -- `replay.py` imports nothing from `harness.py`; all session state comes from reading the JSONL file. Replay produced correct output using only `EventLogReader` + `MockAnthropicClient`. Evidence: `replay.py:1-20` (no harness import), validation PASS output from `run_spike.sh`.
- [x] **Backend endpoints wrap cleanly as `execute()` tools** -- **held with friction** -- Three adapters (`get_match_analysis`, `get_match_history`, `get_steam_player`) dispatched via `tools.py:execute()` and produced the three `tool_result` events at idx 2, 6, 10 in `runs/live_20260413_164013.jsonl`. See friction log below for specific roughness.
- [ ] **Replay against a different model produces coherent output** -- **deferred -- mocked, validation requires follow-up spike with live API**. Mock scripts were scripted to be thematically consistent -- this does not constitute real model validation. Both models were authored by the same hand with the same factual grounding, making coherence trivially guaranteed.
- [x] **Event log survives tool schema change** -- **held** -- Bumped `schema_version` to 2 and added `rank_badge_level` to `FIXTURE_STEAM_PLAYER`. Re-replaying against the original JSONL succeeded; the old tool_result strings are opaque to the replay reader. No migration needed. Evidence: schema_evo.jsonl produced cleanly, run_spike.sh validation PASS.

Accepted assumptions worth flagging: the JSONL format is spike-only and not a contract. If it gets reused without a formal contract review, schema drift in the event envelope (e.g., changing `payload` key names) would be silent.

### Tool wrapping friction log

1. **`get_match_analysis`: positions and damage arrays are enormous and useless as a flat string.** `TransformedMatchData.per_player_data` maps `custom_id -> PlayerMatchData`, where `PlayerMatchData.positions = list[PlayerPosition]` (one entry per sampled second for each of 12 players) and `PlayerMatchData.damage = list[ParsedAttackerVictimMap]` (nested dict keyed by entity_id strings). In a real 39-minute match this would be tens of thousands of elements. The fixture stubs both to one element with a truncation note. A real tool adapter needs explicit field selection -- either a dedicated `get_player_positions(match_id, player_id, window_s)` tool or server-side projection on the analysis endpoint.

2. **`lane_pressure` wave_id keys are opaque without `wave_meta`.** The `LanePressureData.pressure` dict uses keys like `"1_0_0"` (lane_team_spawnsec). A model receiving only the pressure dict cannot interpret the keys without cross-referencing `LaneCreepData.wave_meta`. The flatten helper in `tools.py:_flatten_match_analysis` inlines the wave_meta lookup, but this means the tool adapter carries semantic knowledge about the schema -- a coupling that would break if wave_id format changes.

3. **`get_steam_player` has no Deadlock-specific fields.** `SteamPlayer` (from `steam_account.py`) contains identity fields only: `steamid`, `personaname`, `profileurl`. There is no rank, no hero pool, no recent performance. A coaching session needs a separate "get rank" or "get hero pool" call for any meaningful player context beyond name resolution. `account_id` in `MatchSummary` is a 32-bit steam_id while `SteamPlayer.steamid` is 64-bit -- a model resolving player identity across tools would need to know about the two forms or be told explicitly. `tools.py:_flatten_steam_player:6` notes this explicitly.

4. **Controller creep-gold aggregators are intentionally absent.** Per `MEMORY.md reference_controller_creep_gold_aggregators_unpopulated`: `m_iCreepGold`, `m_iCreepGoldKill`, etc. are always 0 in replay data. Omitting them from fixtures avoids misleading a model that might reason about gold breakdown; fixtures include only `net_worth` and `last_hits`.

5. **`None` entries in `LanePressureData.pressure` lists are meaningful but invisible.** Null pressure snapshots indicate seconds where no alive creep wave existed. A model receiving a flat string "avg_pressure=0.53 over 2 sampled seconds" loses the information that some seconds had null waves. The scripted mock response reflects this (turns 1 and 2) because it was authored to, but a real model needs either the raw list (including Nones) or a "null_seconds" count in the flattened string.

### Coherence diff (model A vs model B)

**Caveat: both models are mocked with hand-scripted responses. Differences below are scripted, not model-driven. This table validates the replay pathway, not real semantic coherence.**

| Turn | Model A (claude-opus-4-6-mock) | Model B (claude-sonnet-4-6-mock) | Coherent? |
|------|-------------------------------|----------------------------------|-----------|
| 1 -- performance overview | "Paradox had a solid match... KDA of 2.0 is above average" | "performed well overall... 8/4/12 with 62k net worth" | yes |
| 2 -- mid-game tempo | "None snapshots... Paradox likely rotated off-lane without a replacement wave" | "None entries (no live creep wave)... common mechanical gap at this skill level" | yes |
| 3 -- lane pressure compare | "both had similar average pressure... Ivy contributed to attributed_players" | "Paradox's duo had the edge... good duo coordination during dives" | yes |
| 4 -- what to work on | "timing wave resets before rotating; converting net-worth lead into objective pressure" | "wave timing before rotations; net-worth advantage into objective damage" | yes |
| 5 -- 3-bullet summary | "wave management on rotations" as priority | "Good KDA but wave management needs work; net worth lead not converted" | yes |

**Divergences from plan:**
1. **Tool surface substitution** -- plan named `get_lane_pressure` (no HTTP route in main) and `get_player_metadata` (no endpoint). Substituted: `get_match_history` (GET /account/match_history/{steam_id}) and `get_steam_player` (GET /account/steam/{steam_id}). `get_match_analysis` retained.
2. **Mocked Anthropic client** -- no `import anthropic`, no real API calls. `MockAnthropicClient` in `mock_client.py` mirrors SDK shape with scripted responses. Session owner approved this constraint.
3. **Stdlib-only harness** -- no pydantic, no httpx, no anthropic SDK. Fixtures are plain dicts. Tool adapter return values are strings (not Pydantic model_dump dicts).

---

## Learnings Output

- [x] Draft entry appended to `private/learnings.md` ## Drafts (covers: event-log shape that survived replay, Pydantic-to-tool-string friction points, mocked-model caveat)
- [x] Follow-up questions or spikes needed:

  **Priority (required to close assumption #3):**
  - **Real-API coherence spike** -- replay `runs/live_20260413_164013.jsonl` against two live Anthropic model IDs (Opus 4.6 and Sonnet 4.6), human-evaluate the 5 turns side-by-side. This is the direct continuation of the current spike and must run before assumption #3 can move from `deferred` to `confirmed`.

  **Deferred candidates (lower priority, not blocking):**
  - Sandboxed code execution spike (pandas over parsed match data) -- if coaches need ad-hoc slicing
  - Vault + Steam OAuth design spike -- before any player-facing deployment
  - Prompt caching cost model -- for production scale
  - Multi-match session spike -- does the event log model survive cross-match coaching history?
  - Tool surface projection spike -- `get_match_analysis` cannot return `positions` / `damage` as flat strings at real match scale; design a `get_player_positions(match_id, player_id, window_s)` or server-side projection before the real coaching tool surface ships.

---

## Out of Scope (do not drift into these)

- Building production MCP servers
- Real player auth
- Frontend integration
- Storage backend beyond a local JSONL file
- Parser cold-start handling
- Cost optimization
- More than 3 tools

If any of these feel necessary to answer the question, stop and flag -- the scope is wrong.

---

## Plan Review

Run `spec-writer` agent after filling in Findings to review: template alignment, confidence labels applied correctly, assumptions checked against findings, learnings drafted, follow-up spikes identified where confidence is below `confirmed`, and scope discipline (did the spike drift into out-of-scope territory?).

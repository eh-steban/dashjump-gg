# Position Timeline Alignment Investigation

**Status:** In progress — critical misalignment discovered between API data, haste inspector, and parser output

## Key Findings

### 1. Coordinate System & Transformation
- **World bounds:** x, y ∈ [−10752, +10752]
- **Cell-based encoding:** Position = `cell * 512 - 16384 + vec`
  - `cell` (uint16): grid cell index
  - `vec` (quantized float): offset within cell
  - Example: cellX=32, vecX=0 → world_x = (32×512−16384)+0 = 0 (map center)
- **Normalization:** `normX = (x+10752)/21504`, `normY = 1−(y+10752)/21504`

### 2. Deadlock API Match Metadata
- **Player slot 3, first 30 seconds** (from `match-metadata-response.json` for match 55423930):
  - x_min=−9147, x_max=9186, y_min=−10103, y_max=8146
  - x_resolution=16383, y_resolution=16383, interval_s=1.0
  - **Samples 0−7:** Player stationary at x≈−270, y≈−10057 (Amber base, deep bottom-center)
  - **Sample 8:** Player starts moving toward lane (begins travel)
  - **Sample 24+:** Player in lane
  - Ground truth that other sources should align with

### 3. Haste Inspector Data (Match 55423930, Player Slot 2)
- **Tick 1152 (demo 18s):** cellX=28, vecX=535.75 → world_x≈920, cellY=24, vecY≈359 → world_y≈1831 (center-map area, NOT base)
- **Tick 1216 (demo 19s):** world_x≈1236, world_y≈2565 (still center-map, NOT base)
- **Tick 1280 (demo 20s):** world_x≈1451, world_y≈2670 (still NOT base)
- **Tick 1344 (demo 21s):** world_x≈−440, world_y≈3175 (still NOT base)
- **Observation:** Player slot 2 is NOT in Amber base (x≈−270, y≈−10057) in this tick range

### 4. Parser Output Findings
- **Match start calculation:** Uses `m_flGameStartTime` field → rounds to 18 or 38 seconds (depending on replay)
- **Tick interval:** 64 ticks per second (tick_interval=0.015625s)
- **Position collection:** One frame per second (when `next_window != this_window`)
- **Game-relative indexing:** `match_window = this_window - match_start_time_s`
  - Positions stored 0-indexed from game second 0
  - **Total frames collected:** 2431 (replay 2), 3068 (replay 1)

### 5. Base Coordinate Search Results
- **Replay 2 (match_start=38):** Earliest base entry at demo_s=120, game_s=82 (FAR TOO LATE)
  - Expected: demo_s≈39-40, game_s≈1-2
  - **Gap of ~80 seconds** between expected and actual
- **Replay 1 (match_start=18):** Earliest base entry at demo_s=19.2, game_s=79 (also too late)
  - Expected: demo_s≈19-20, game_s≈0-1
  - But at least demo_s is roughly correct!

### 6. Critical Mismatches Identified
1. **API vs Haste:** Deadlock API shows player slot 3 in base; haste shows player slot 2 NOT in base at same ticks
2. **API vs Parser:** Deadlock API shows base coordinates x≈−270, y≈−10057; parser finds those coords in game_s≈82+ (80+ seconds late)
3. **Player slot numbering:** API uses "player_slot", haste shows arbitrary entities, parser uses "custom_id" — may not be the same mapping

## Unanswered Questions
- Why does parser first find base coordinates at game_s=82 when they should appear at game_s=0-8?
- Are we collecting positions from the wrong entity indices?
- Is `custom_id` actually player slot, or some other numbering?
- Does the API use a different player numbering scheme than the demo?

## Next Steps
1. Verify player slot mapping: confirm parser's `custom_id` matches API's `player_slot`
2. Check if we're extracting positions from correct entities (check entity hashes/types)
3. Compare early frame (game_s=0) against haste data at demo_s=18-19
4. Investigate why game_s=82 is first base coordinate — possible off-by-one or frame skip in collection?

# Plan: Move Lane Pressure Into Player Cards

## Context

`LanePressurePanel` is a standalone panel in the right column of the match analysis page, sitting above the minimap. It shows per-lane creep wave pressure with player attribution for the current tick. The user wants this data surfaced directly on each player card instead — with a persistent "Lane Pressure" field that's always visible, showing `Lane X: XX%` when the player is attributed to a wave, or `—` when they're not.

A future **Lane Card** component will also need lane pressure data and player data (name, hero image), so the attribution lookup logic must be extracted as a reusable utility rather than inlined in `PlayerCards`.

## Outcome

- Lane pressure attribution is visible per player at a glance, inline with their card
- The right column has more vertical space for the minimap (LanePressurePanel removed)
- The `LanePressurePanel.tsx` file is deleted
- Attribution lookup lives in a shared service, ready for Lane Cards to consume

---

## Key Format Note

The pressure record keys follow the format `{lane}_{team}_{waveIdx}` (e.g., `"1_2_0"`, `"1_2_1"`). Multiple wave indices for the same lane+team represent concurrent waves — e.g., if a first wave is ignored, a second spawns behind it. When parsing a key, split on `_` and take index 0 (lane), index 1 (team), index 2 (waveIdx).

For display, **group by lane+team** (dropping waveIdx). If a player is attributed to any wave in a lane+team, show that lane. If attributed to multiple waves in the same lane (both `1_2_0` and `1_2_1`), pick the leading wave (highest pressure) for the display value.

---

## What the Card Will Look Like

**When attributed:**
```
| [Hero Img]  Abrams
|             (Name: SteelFist, Slot: 1, Team: 2)
| ─────────────────────────────────────────────── |
| Health: -
| Current Region: Lane 1
| Lane Pressure: Lane 1 · 73%         ← amber-colored (#FFA500)
|
| Out of combat
```

**When not attributed (persistent field):**
```
| Lane Pressure: —                    ← gray dash
```

If attributed to waves in multiple lanes: `Lane 1 · 73%, Lane 2 · 42%`.

Team color comes from the **wave's** team (parsed from the key), not the player's team — handles edge cases where a player is near the enemy's wave.

---

## Files to Change

### 1. NEW: `frontend/src/services/lanePressure/index.ts`

Extract the attribution lookup into a standalone service function so both `PlayerCards` and the future `LaneCard` component can share it without duplication.

```typescript
import { LanePressureData } from '../../domain/lanePressure';

export interface PlayerLanePressureEntry {
  lane: string;
  team: number;
  pressure: number; // 0–1
}

/**
 * Returns the lane pressure entries attributed to a player at a given tick.
 * Groups by lane+team, keeping the highest-pressure wave per group.
 */
export function getPlayerLanePressure(
  playerCustomId: number,
  lanePressure: LanePressureData,
  currentTick: number
): PlayerLanePressureEntry[] { ... }
```

Logic: iterate `lanePressure.pressure` entries, parse key as `{lane}_{team}_{waveIdx}`, check `snapshot.attributed_players.includes(playerCustomId)`, group by `{lane}_{team}`, keep highest pressure per group.

### 2. `frontend/src/components/matchAnalysis/PlayerCards.tsx`

**What changes:**
- Add `lanePressure: LanePressureData` to `PlayerCardsProps`
- Import `LanePressureData` from `../../domain/lanePressure`
- Import `getPlayerLanePressure` from `../../services/lanePressure`
- Inside the player `.map()` loop, call the service:
  ```typescript
  const pressureEntries = getPlayerLanePressure(Number(player.custom_id), lanePressure, currentTick);
  ```
- Add a `Lane Pressure` row in the card JSX:
  - `pressureEntries.length === 0` → `<span className="text-gray-400">—</span>`
  - Otherwise → each entry formatted as `Lane {lane} · {Math.round(pressure * 100)}%` in team color (`#FFA500` amber / `#0EA5E9` sapphire)

### 3. `frontend/src/pages/MatchAnalysis.tsx`

**What changes:**
- Remove the `LanePressurePanel` import (line 9)
- Remove the `<LanePressurePanel ... />` JSX block (lines 266–270)
- Add `lanePressure={parsedMatchData.lane_pressure}` prop to `<PlayerCards>`

### 4. `frontend/src/components/matchAnalysis/LanePressurePanel.tsx`

**Action: Delete this file.** No longer rendered, no other usages.

---

## Verification

After implementation:

1. **Build check**: `cd frontend && npm run build` — must complete without TypeScript errors
2. **Visual check**: Open the match analysis page for any match ID
   - Player cards show a "Lane Pressure" row (always present)
   - At ticks when a player is near a creep wave: `Lane 1 · 73%` in team color
   - At other ticks: `—` in gray
   - The separate LanePressurePanel box is gone from the right column
3. **Edge cases**: Verify a tick where both `1_2_0` and `1_2_1` are active — player should only see one entry for Lane 1 (the higher-pressure wave)

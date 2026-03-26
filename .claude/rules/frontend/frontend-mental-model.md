---
paths:
  - "frontend/src/**/*.ts"
  - "frontend/src/**/*.tsx"
  - "frontend/src/**/**/*.ts"
  - "frontend/src/**/**/*.tsx"
---
# Frontend Mental Model

## Module Structure

```
frontend/
├── src/
│   ├── api/                          # API Clients
│   │   └── matchAnalysis.ts
│   │
│   ├── domain/                       # Domain Models (mirrors backend)
│   │   ├── matchAnalysis.ts
│   │   ├── player.ts
│   │   ├── boss.ts
│   │   ├── timeline.ts
│   │   └── ...
│   │
│   ├── services/                     # Business Logic
│   │   ├── damage/
│   │   │   ├── index.ts
│   │   │   ├── aggregation.ts
│   │   │   └── helpers.ts
│   │   └── timeline/
│   │       └── index.ts
│   │
│   │
│   ├── components/                   # Feature-grouped Components
│   │   ├── matchAnalysis/
│   │   │   ├── MatchTimeViewer.tsx
│   │   │   ├── Minimap.tsx
│   │   │   ├── PlayerCards.tsx
│   |   |   ├── hooks/                        # React Hooks
│   |   |   │   └── useMatchAnalysis.ts
│   │   │   └── ...
│   │   ├── damageAnalysis/
│   │   │   ├── PlayerDamageDistribution.tsx
│   │   │   └── ...
│   │   ├── profile/
│   │   └── login/
│   │
│   ├── pages/                        # Route-level Components
│   │   ├── MatchAnalysis.tsx
│   │   └── ...
│   │
│   ├── data/                         # Static Data
│   │   ├── combatTypes.ts
│   │   ├── regions.ts
│   │   └── ...
│   │
│   ├── utils/                        # Utilities
│   │   └── time.ts
│   │
│   ├── App.tsx
│   ├── index.tsx
│   └── vite-env.d.ts                 # TypeScript defs for Vite env vars
│
├── tests/                            # Mirrors src/ structure
│   ├── setup.ts                      # Global test setup (cleanup)
│   ├── api/
│   ├── domain/
│   ├── services/
│   ├── components/
│   └── pages/
│
├── Dockerfile
├── package.json
└── vite.config.ts                    # Includes Vitest config in `test` block
```

> **Navigation note:** This shows the target architecture. Current layout diverges:
> `matchAnalysis/` has 15+ components (CreepWave, LanePressure, Objectives, etc.) · No `hooks/` subdir yet · `domain/` has additional types (boss, creep, lanePressure, matchMetadata, etc.) · Required env var: `VITE_BACKEND_DOMAIN`

---

## Candidate Topics

### Timeline Scrubbing Architecture
The match timeline allows coaches to scrub through game time. When this is built out, document:
- How game_start_marker is used to convert user-selected time to position index
- How the minimap canvas updates on scrub
- How position data is loaded/cached for smooth scrubbing
- See `.claude/rules/parser/parser-mental-model.md` for the conversion formula

**Pre-wire:** Before implementing, load `private/learnings.md#demo-timeline-offset-reconciliation-pattern`.

### CreepWaveLayer: Per-Creep Rendering Architecture

**Added:** March 2026 (Phase C of lane creep tracking refactor)

The minimap renders creeps as individual dots (per-creep positions) plus wave-death pins. Key design decisions:

**Two-pass rendering.** Pass 1 iterates `laneCreepData.creeps` (entity_index → timeline) for live dots. Pass 2 iterates `laneCreepData.wave_meta` (wave_id → meta) for pins. The passes are independent -- pins do not need to cross-reference entity snapshots.

**Pin supersession via `last_death_sec`.** A pin shows at a wave's `last_death_x/y` when `last_death_sec` has passed. It is superseded (hidden) when a newer wave on the same `(lane, team)` also has `last_death_sec` set and that second has passed. Comparison: newer wave = `spawn_sec > current_wave.spawn_sec`. At most one pin is visible per lane+team at any time. Extracted as a pure `isPinVisible(waveId, meta, currentSec, allMeta)` function for unit testability.

**`currentSec` vs `currentTick`.** Both represent seconds elapsed since match start in the current architecture. `CreepWaveLayer` accepts both as separate props (`currentTick` for timeline array indexing, `currentSec` for pin visibility) to make intent explicit and allow future divergence with fractional ticks.

**Coordinate space is identical to player positions.** Per-creep positions use `x/y` in world units with the same `WORLD_BOUNDS`. The `worldToMinimapPixels` function is reused without modification. No special offset needed for creep coordinates.

### Canvas vs SVG Decision Points
The visualization philosophy uses Canvas for performance-critical rendering (player positions, trails) and SVG for interactive elements (objectives, annotations). Document gotchas here as they emerge.

### Game Phase Filtering State
When phase filtering is implemented, document:
- How current phase is tracked across components
- How filtering affects timeline position data
- How phase boundaries are determined from match data

### Component Composition Patterns
(Populate as non-obvious patterns emerge across the matchAnalysis/ and damageAnalysis/ feature groups)

---

**See `.claude/knowledge-management.md` for when and how to populate this file.**

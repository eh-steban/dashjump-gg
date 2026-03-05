---
paths:
  - "frontend/src/**/*.ts"
  - "frontend/src/**/*.tsx"
  - "frontend/src/**/**/*.ts"
  - "frontend/src/**/**/*.tsx"
---
# TypeScript Standards

## General

- TypeScript strict mode enabled
- Avoid `any` — use `unknown` with type guards
- Type all props, state, and API responses
- Infer types from backend schemas where possible

## Interface vs Type

**Prefer `interface` over `type`** unless using an interface is overkill.

Reasons:
- Better error messages
- More flexible when extending
- Declaration merging when needed

```typescript
// ✅ Preferred - interface for object shapes
interface Player {
  id: number;
  name: string;
  heroId: number;
}

interface MatchAnalysis {
  matchId: number;
  players: Player[];
  duration: number;
}

// ✅ Extend with interface
interface DetailedPlayer extends Player {
  stats: PlayerStats;
}

// ✅ Type is fine for unions, primitives, utilities
type TeamSide = 'amber' | 'sapphire';
type PlayerId = number;
type PlayerMap = Record<PlayerId, Player>;

// ✅ Type is fine for simple function signatures
type DamageCalculator = (raw: number, armor: number) => number;
```

## Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Files (general) | camelCase | `matchAnalysis.ts`, `useMatchData.ts` |
| Files (components) | PascalCase | `PlayerCards.tsx`, `Minimap.tsx` |
| Interfaces | PascalCase | `MatchAnalysis`, `PlayerStats` |
| Types | PascalCase | `TeamSide`, `DamageEvent` |
| Constants | UPPER_SNAKE_CASE | `API_BASE_URL`, `MAX_PLAYERS` |

## Null Handling

- Prefer `undefined` over `null` for optional values
- Use optional chaining (`?.`) and nullish coalescing (`??`)
- Be explicit about nullable types

```typescript
// ✅ Good
interface MatchState {
  currentTime?: number;  // Optional, may be undefined
  selectedPlayer: Player | null;  // Explicitly nullable
}

// ✅ Good - explicit handling
const playerName = match.players.find(p => p.id === id)?.name ?? 'Unknown';
```

## Generics

Use generics for reusable utilities, but don't over-engineer:

```typescript
// ✅ Good - useful generic
function groupBy<T, K extends string | number>(
  items: T[],
  keyFn: (item: T) => K
): Record<K, T[]> { ... }

// ❌ Overkill for simple cases
function getPlayerName<T extends { name: string }>(player: T): string {
  return player.name;  // Just use Player type directly
}
```

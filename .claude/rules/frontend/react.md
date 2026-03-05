---
paths:
  - "frontend/src/**/*.ts"
  - "frontend/src/**/*.tsx"
  - "frontend/src/**/**/*.ts"
  - "frontend/src/**/**/*.tsx"
---
# React Patterns

## Component Organization

Feature-based organization, not by file type. Colocate related components, hooks, and types.

```
src/components/
├── matchAnalysis/
│   ├── index.ts                  # Public exports
│   ├── MatchTimeViewer.tsx
│   ├── Minimap.tsx
│   ├── PlayerCards.tsx
│   ├── hooks/
│   │   └── useMatchData.ts
│   └── types.ts
├── damageAnalysis/
│   ├── PlayerDamageDistribution.tsx
│   └── ...
```

## Component Structure

```tsx
// matchAnalysis/index.ts
export { MatchAnalysis } from './MatchAnalysis';

// matchAnalysis/MatchAnalysis.tsx
import { useMatchData } from './hooks/useMatchData';
import { DamageChart } from './components/DamageChart';

export const MatchAnalysis: React.FC<Props> = ({ matchId }) => {
  // Logic here
};
```

## State Management

| State Type | Approach |
|------------|----------|
| UI-only state | `useState` |
| Shared feature state | React Context |
| Server state | React Query (recommended) |

## Props

- Define props interface in component file or colocated `types.ts`
- Use destructuring with defaults where appropriate
- Prefer composition over prop drilling

```tsx
// ✅ Good
interface PlayerCardProps {
  player: Player;
  isSelected?: boolean;
  onSelect?: (playerId: number) => void;
}

export const PlayerCard: React.FC<PlayerCardProps> = ({
  player,
  isSelected = false,
  onSelect,
}) => { ... };
```

## Hooks

- Prefix with `use`
- One concern per hook
- Return object for multiple values (easier to extend)

```tsx
// ✅ Good
export function useMatchData(matchId: string) {
  // ...
  return { match, isLoading, error, refetch };
}
```

## UI Components (Headless UI)

Use `@headlessui/react` for accessible, unstyled UI primitives. Style with Tailwind.

**When to use:**
- Tabs, modals, menus, dropdowns, comboboxes
- Any component requiring keyboard navigation and ARIA attributes

```tsx
import { Tab } from '@headlessui/react';

<Tab.Group defaultIndex={0}>
  <Tab.List className="flex border-b border-gray-200">
    <Tab className={({ selected }) =>
      `px-4 py-2 ${selected ? 'border-b-2 border-blue-500 text-blue-600' : 'text-gray-500'}`
    }>
      Tab Label
    </Tab>
  </Tab.List>
  <Tab.Panels>
    <Tab.Panel>Content</Tab.Panel>
  </Tab.Panels>
</Tab.Group>
```

**Benefits:**
- Built-in accessibility (ARIA roles, keyboard navigation)
- Unstyled — full control with Tailwind
- From the Tailwind team, integrates seamlessly

## Styling (Tailwind)

- Use utility-first approach
- Group classes: layout → spacing → typography → colors → effects
- Extract repeated patterns to component variants

```tsx
// ✅ Good - logical grouping
<div className="flex items-center gap-4 p-4 text-sm text-gray-700 bg-white rounded-lg shadow">
```

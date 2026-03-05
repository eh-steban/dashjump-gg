---
paths:
  - "frontend/src/**/*.ts"
  - "frontend/src/**/*.tsx"
  - "frontend/src/**/**/*.ts"
  - "frontend/src/**/**/*.tsx"
---
# Data Visualization Guidelines

## Core Principle

**Visualizations should tell a story, not just display data.**

Focus on actionable insights for coaches — what decision does this help them make?

## Design Approach

1. Start with conventional, proven chart types
2. Defer complex custom visualizations until validated by users
3. Prioritize clarity over visual impressiveness

## Game Phase Integration

See root `.claude/CLAUDE.md` for game phase definitions. Visualizations should:
- Allow filtering/navigation by phase
- Highlight phase transitions
- Aggregate metrics per phase when appropriate

## Dual-Axis Timeline Pattern

Inspired by financial charts (candlesticks + volume). Useful for match timelines:

```
┌─────────────────────────────────────────┐
│  Key Events & Phases                    │  ← Top: discrete events
│  ○ Kill  ◆ Objective  │ Phase Marker    │
├─────────────────────────────────────────┤
│  ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~    │  ← Bottom: continuous metrics
│  Gold / Damage / etc.                   │
└─────────────────────────────────────────┘
```

- Top axis: Key events, objectives, team fights, phase transitions
- Bottom axis: Continuous metrics (gold differential, damage, etc.)
- Handle 25-45 minute matches without visual clutter

## Performance Guidelines

| Data Type | Recommended Approach |
|-----------|---------------------|
| High-frequency (positions) | Canvas rendering |
| Interactive elements | SVG |
| Large datasets | Virtualization, time-based filtering |

## Rendering Technology

```tsx
// Canvas for performance-critical rendering
// Player positions, trails, heatmaps
<canvas ref={canvasRef} />

// SVG for interactive elements
// Objectives, annotations, clickable markers
<svg>
  <circle onClick={handleObjectiveClick} />
</svg>
```

## Time-Based Navigation

- Support scrubbing through match timeline
- Maintain context during navigation (filters, selections)
- Show loading states for data-heavy time ranges

## Accessibility

- Don't rely solely on color to convey information
- Provide text alternatives for key insights
- Support keyboard navigation for interactive elements

## Chart Library Guidance

- Use established libraries (Recharts, D3) for standard charts
- Custom canvas rendering for specialized match visualizations
- Consistent styling across all visualizations

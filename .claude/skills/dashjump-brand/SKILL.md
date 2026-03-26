---
name: dashjump-brand
description: dashJump.gg brand identity and design system. Apply these guidelines for any frontend visual work, component styling, or UI design decisions.
---

# dashJump.gg Brand Identity

## Brand Positioning

Esports analytics for competitive Deadlock coaches. Aesthetic direction: data-dense terminal meets gaming culture -- high-information density, dark-first, no decorative chrome.

## Design Principles

- **Data-dense but clean** -- every pixel earns its place; no decorative elements
- **Story-first visualizations** -- tell a story with data, don't just display it
- **Game-phase aware** -- layouts and highlights shift by phase (laning / mid / late)
- **Dark mode primary**, light mode secondary
- **Colorblind accessible** -- critical for data visualizations; never rely on color alone

## Tech Stack

- React + TypeScript (strict mode)
- Tailwind CSS with utility classes (no arbitrary CSS values)
- shadcn/ui as component foundation
- Recharts for all chart/visualization primitives

## Color System

**Status: TBD** -- pending brand discovery experiment (see `private/product/experiments/`).

Until defined, use Tailwind's zinc/slate scale for neutrals and a single blue or cyan accent. Avoid saturated reds/greens as primary data colors (colorblind accessibility).

Data visualization palette: use Recharts defaults or a manually curated 8-color accessible palette. Never use fewer than 3 lightness steps between adjacent series colors.

## Component Patterns

### Data containers
- Use `rounded-lg border border-zinc-800 bg-zinc-900` as the base card surface
- Section headers: `text-sm font-semibold text-zinc-400 uppercase tracking-wide`
- Data values: monospace font, tabular-nums

### Loading and error states
Every component must have both. Use skeleton shimmer for loading, not spinners. Error states show a brief message with a retry affordance.

### Responsive breakpoints
Follow Tailwind defaults. Minimum supported: `sm` (640px). Data-dense views may collapse to a scrollable list on mobile.

### Dark / light variants
Use `dark:` prefix. Dark is the default; always implement dark variant first.

## Voice & Tone

Concise and technical. Coach-first language -- "lane pressure", "objective control", not "player positions near towers". Numbers over adjectives. No filler copy.

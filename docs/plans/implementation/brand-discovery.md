# Brand Discovery Plan — dashJump.gg
**Created:** 2026-02-28
**Owner:** Steven Rodriguez
**Status:** In Progress — Step 1 next

---

## Background

dashJump.gg is a professional-grade analytics platform for Deadlock esports coaches. It is in early validation — two coach partners confirmed, building toward paid adoption. The platform computes derived stats from replay parsing (lane priority, solo time, fight classification, hero matchup) that API competitors cannot match.

This brand discovery establishes dashJump's first-iteration visual identity. It must be:
- **Respectable**: Coaches and the Deadlock esports community take it seriously on first glance
- **Honest**: Not over-polished — serious product being built by people who understand the space
- **Functional**: Every design decision serves coaches reaching insights fast

The primary user is a Deadlock coach during or before a film review session. They need to reach a specific analytical insight within 60 seconds of opening the page.

---

## Brand Concept: "Iron Precision"

Derived from the dashjump movement tech metaphor: *perfect the fundamentals*. Great players don't perform — they make high-value decisions. dashJump enables that. The visual language should feel like a film room for elite coaches, not a product page for fans.

**Discovery interview inputs:**
- Aesthetic direction: Esports-forward (embraces gaming DNA, still clean and professional)
- Reference: Sports analytics tools (Opta, StatHead, Wyscout) — signals serious intent
- Brand feelings: Credible, Precise, Distinctive
- Name philosophy: "Perfect the fundamentals" — dashjump is a basic movement tech; analytics that enables high-value decisions, not flashy ones
- Color preferences: Purple (premium callouts) and green (positive data, momentum)

---

## Constraints (Fixed — Cannot Change)

- **Team colors must coexist**: Amber and Sapphire appear on every match analysis page. Brand palette must not conflict or compete with these.
- **Dark mode default**: Esports audience expects dark themes. Dark is the primary experience.
- **Light mode required**: User-selectable, persisted preference (localStorage). System `prefers-color-scheme` used as initial default before user sets preference.
- **Responsive**: Desktop/tablet primary. Mobile fallback — content accessible, analytics-optimal only at ≥768px.
- **WCAG AA minimum**: 4.5:1 contrast for normal text, 3:1 for large text and UI components. AAA where feasible.

---

## Design System Requirements

### Table Stakes Standards (full list)

#### Must Have for v1

| Standard | What It Means | Why It Matters |
|----------|--------------|----------------|
| **Surface Stack** | 4 background levels per mode (base, card, elevated, overlay) | Depth without decoration; cards float above page without borders |
| **WCAG Contrast** | Text: ≥4.5:1, large text/UI: ≥3:1 | Accessibility + reputability; colorblind coaches must be able to read data |
| **Focus State System** | Single focus-ring token applied to all interactive elements | WCAG 2.4.7; keyboard navigation; judges product quality immediately |
| **`prefers-reduced-motion`** | All animations have a static fallback | Accessibility for vestibular/epilepsy; also makes gradient data components (Lane Cards) safe |
| **Interactive State Vocabulary** | Default, hover, active, focus, disabled for all interactive elements | Inconsistency here is one of the clearest signs of an unfinished product |
| **Loading / Skeleton States** | Skeleton screens for async data components | Match data takes time to parse; spinners feel unfinished, skeletons feel intentional |
| **Error State Design** | Branded treatment for ErrorMessage component | Current component is functional but unstyled |
| **Empty State Patterns** | No matches, no data, first-time experience designs | These moments define whether the product reads as complete or abandoned |

#### Should Have for v1

| Standard | What It Means | Why It Matters |
|----------|--------------|----------------|
| **Elevation System** | Shadow levels 0–4 complementing surface stack | Surfaces + shadows together = real depth; shadows alone on flat bg look cheap |
| **Border Radius Tokens** | `none`, `sm=4px`, `md=8px`, `lg=12px`, `full` | Inconsistent radii are one of the most visible polish gaps |
| **Icon Style Standard** | Stroke weight, size grid (16/20/24px), fill vs outline rule, library choice | Pick one library (Lucide recommended for React) and document usage |
| **Z-Index Hierarchy** | Defined layers: dropdown=100, tooltip=200, modal=300, notification=400 | Prevents stacking bugs as feature count grows |
| **Typography Scale** | Mathematical modular scale (1.25 "major third" ratio) between heading sizes | Ensures visual harmony; not just font choices but size relationships |

#### Nice to Have (flag, not block)

| Standard | Notes |
|----------|-------|
| **Scrollbar Styling** | WebKit only, degrades gracefully; thin styled scrollbar reads as intentional on dark themes |
| **Print Styles** | `@media print` basics; coaches may print match reports |
| **`prefers-color-scheme`** | Auto-detect OS preference as initial value before user sets in-app preference |

---

## Color System Architecture

Five distinct layers. All defined for both dark and light modes.

### Layer 1: Surface Stack (Background Plan)
Foundational. Creates visual hierarchy through background depth rather than borders — standard in professional design systems (Material Design, Linear, Vercel, Figma).

| Token | Dark Value | Light Value | Usage |
|-------|-----------|-------------|-------|
| `surface-base` | ~`#0B0C10` | ~`#F4F5F7` | Page canvas, darkest/lightest |
| `surface-1` | ~`#141519` | ~`#FFFFFF` | Default card/panel |
| `surface-2` | ~`#1E2028` | ~`#F0F1F5` | Elevated card, dropdown |
| `surface-overlay` | ~`#282A35` | ~`#E8E9EF` | Modal background, tooltip |

*Exact values selected in Step 1 based on chosen primary color's undertone.*

### Layer 2: Brand Palette

Purple as primary — distinguished from every other esports analytics tool ([redacted competitor], dotabuff all use blue/neutral). Sophisticated violet reads premium, not "gaming purple." Does not conflict with Amber or Sapphire team colors.

Green as secondary — serves double duty: brand secondary color (growth, insight, momentum) *and* semantic success color. Intentional overlap simplifies the system.

| Role | Candidate | Usage |
|------|-----------|-------|
| **Primary** | Purple `~#7C5CBF` | Brand identity, CTAs, brand mark, active nav |
| **Secondary** | Green `~#4ADE80` (muted for dark) | Positive data signals, improvements, success states |
| **Tertiary** | Gold `~#D4A839` | Premium callouts, achievement moments, Deadlock industrial nod |

*Exact shades selected in Step 1 exploration.*

### Layer 3: Team Colors (Fixed, Contextual)
These are data colors, not brand colors. Appear in match analysis contexts only.

| Color | Role | Usage |
|-------|------|-------|
| Amber | Deadlock team | Team labels, player cards, lane pressure gradient |
| Sapphire | Deadlock team | Team labels, player cards, lane pressure gradient |

### Layer 4: Semantic Colors

| Semantic | Color | Note |
|----------|-------|------|
| Success | Green (same as secondary — intentional) | Works at data scale and notification scale |
| Warning | Amber-orange | Chosen to be distinguishable from Amber team color — slightly more orange, less yellow |
| Error | Rose/red | Standard; no conflict risk |
| Info | Neutral blue | Lighter than Sapphire team blue; muted |

### Layer 5: Data Visualization Palette
12+ distinct colors for Sankey diagrams, timelines, charts.
- Colorblind accessible: no red/green-only pairs for critical distinctions (use shape + label as fallback)
- Work on both dark and light surfaces
- Distinct from team colors and semantic colors
- Deuteranopia and protanopia safe

*Palette values defined in Step 2 after primary/secondary color selections.*

---

## New Component Pattern: Gradient Data Components

The Lane Cards component introduces a new component category: **gradient data components**, where the background surface itself carries data meaning.

### Lane Cards Specification

**Concept**: Each lane has a card showing who is in the lane. The card background is a left-to-right gradient (Amber on left → Sapphire on right). The center point of the gradient shifts based on lane pressure — if Sapphire has 70% pressure, the sapphire color bleeds 70% across the card.

**States:**
- Amber dominant (>50% pressure): gradient center right of midpoint, amber color dominates
- Balanced (~50%): pure center gradient
- Sapphire dominant: gradient center left of midpoint, sapphire color dominates

**Text contrast problem**: Text placed directly on a gradient fails WCAG on one or both sides. Solution: a semi-transparent dark scrim layer (`background: rgba(0,0,0,0.45)`) over the gradient, behind the content. The content surface is readable while the gradient shows through.

**`prefers-reduced-motion`**: If gradient transition animates on tick change, a static snapshot is shown for users who have reduced motion enabled.

**Step 1 preview must include**: Lane Cards shown at three states (amber dominant, balanced, sapphire dominant) with scrim contrast solution visible.

---

## Typography System

Three font roles with a 1.25 modular scale between heading sizes:

| Role | Purpose |
|------|---------|
| **Display** | Match headers, stat highlights, brand mark. Condensed, geometric, technical. |
| **Body** | Labels, descriptions, UI copy. Legible at small sizes. |
| **Data/Mono** | Stat values, timestamps, match IDs, table numbers. Monospace for alignment. |

Three candidate pairings shown in Step 1 exploration.

---

## Responsive Strategy

Desktop-primary with graceful degradation. Analytics density is optimal at `lg` and above.

| Breakpoint | Width | Target |
|------------|-------|--------|
| `sm` | ≥640px | Minimum accessible view |
| `md` | ≥768px | Tablet — all features accessible, some condensed |
| `lg` | ≥1024px | Primary target — full analytics layout |
| `xl` | ≥1280px | Optimal — side-by-side panels |
| `2xl` | ≥1536px | Enhanced — more data visible |

Mobile (<640px): Accessible, not analytics-optimal. Match summary and key stats readable; minimap/complex panels stack vertically.

---

## Steps

### Step 1: Font + Color Exploration Page ← CURRENT
**Owner:** Claude (dashjump-designer)
**Deliverable:** `private/brand-preview-options.html` — standalone HTML, opens in any browser, no build step

Contents:
- **3 font pairings**, each rendered with realistic content (match header, stat label+value, body text, nav item):
  - Pairing A: Rajdhani (display) + Outfit (body) + IBM Plex Mono (data)
  - Pairing B: Barlow Condensed (display) + DM Sans (body) + Fira Code (data)
  - Pairing C: Exo 2 (display) + Source Sans 3 (body) + JetBrains Mono (data)
- **4 primary color options**, each shown on dark and light surface stack with realistic UI content:
  - Purple: `~#7C5CBF` — brand-distinctive, premium, doesn't conflict with team colors
  - Purple + Green accent: purple primary, green secondary shown together
  - Deep Green primary: `~#22C55E` (muted) — growth/momentum as primary identity
  - Teal-Cyan: `~#2ECBD8` — HUD overlay feel, precise fallback option
- **Dark/light mode toggle** — all options shown in both modes
- **Lane Cards preview** — gradient component at three states (amber dominant, balanced, sapphire dominant) with scrim contrast solution
- **Team color check** — Amber and Sapphire shown alongside each primary color option to verify non-conflict

**Decision needed:** Font pairing selection + primary color direction

---

### Step 2: Brand System Specification
**Depends on:** Step 1 decisions
**Owner:** Claude (dashjump-designer)
**Deliverable:** `.claude/skills/dashjump-brand/SKILL.md`

Full coverage:
- Positioning statement + concept
- Full surface stack (dark + light, hex + CSS vars)
- Brand palette (primary/secondary/tertiary + tints/shades for all interactive states)
- Team colors with usage rules and contextual scope
- Semantic colors (4)
- Data visualization palette (12 colors, colorblind notes, swatch reference)
- Typography system (3 fonts, modular scale, weights, line heights, letter spacing, usage rules)
- Spacing system (4px base unit, scale)
- Layout breakpoints
- Elevation system (shadow levels 0–4)
- Border radius token set
- Icon style standard (library, stroke weight, size grid, fill/outline rules)
- Z-index hierarchy
- Interactive state vocabulary (all 5 states)
- Focus state system
- Loading/skeleton state spec
- Empty state patterns
- Error state design
- Gradient data component specification (Lane Cards pattern)
- Dark/light mode switching convention (`data-theme` on `<html>`)
- `prefers-color-scheme` initial value behavior
- `prefers-reduced-motion` rules
- Voice & tone
- Motion principles (easing curves, duration ranges, what to animate)
- Scrollbar styling (WebKit)
- Print style notes

---

### Step 3: Visual Component Preview
**Depends on:** Step 2 complete
**Owner:** Claude (dashjump-designer)
**Deliverable:** `private/brand-preview.html` — standalone HTML with dark/light toggle

Renders:
- Color palette (all tokens, hex, CSS var names)
- Typography specimens (each font, each size, dark + light)
- Surface stack demo (all 4 levels visible)
- Elevation system (shadow levels)
- Interactive states (button in all 5 states)
- 5 UI components in brand style:
  - Match header bar
  - Stat card (Lane Priority, Amber 63% / Sapphire 37%)
  - Lane Cards (all three gradient states)
  - Player card (hero image, name, key stats)
  - Navigation / sidebar
- Loading skeleton example
- Empty state example

---

### Step 4: Tailwind Token Setup
**Depends on:** Step 2 + Step 3 approved
**Owner:** frontend-react agent
**Deliverable:** `frontend/src/index.css` CSS custom properties + Tailwind config extension

All brand tokens as `--dj-*` CSS variables (namespaced).
Dark/light via `data-theme="dark|light"` on `<html>`.
Includes scrollbar styling.

---

### Step 5: Theme Toggle + Key Entry Points
**Depends on:** Step 4
**Owner:** frontend-react agent
**Scope:**
- Theme toggle component (reads OS preference initially, persists user selection to localStorage)
- Login page brand application
- Basic layout shell / navigation
**Deferred:** Full component re-skin of existing analytics components

---

## Implementation Priority Discussion

**When:** Step 3 → Step 4 handoff (before briefing the frontend-react agent)

**User intent:** All "should haves" included; at least 1 "nice to have."

**Sequencing principle:** Things expensive to retrofit go first; things cheap to add later go last.

**Key nuance to address at that discussion:**
Some should-haves are near-zero cost at the *token level* but real work at the *application level*:
- Token: define the CSS variable or media query wrapper in Step 4 (free)
- Application: use that token consistently across all components (per-component work, sequenced later)

Examples of "define now, apply incrementally":
- `prefers-reduced-motion` — one media query wrapper in the token file
- `prefers-color-scheme` — one media query block alongside `data-theme`
- Z-index hierarchy — just CSS variables

Examples of "real implementation work per component":
- Interactive state vocabulary (hover/focus/disabled) — each component
- Skeleton states — each async component needs its own skeleton
- Empty state designs — each data-bearing component

At Step 4 handoff: produce an ordered implementation list for the agent, distinguishing "token-level" from "component-level" work.

---

## Out of Scope (This Plan)
- Logo/wordmark in vector format
- Full component re-skin across all existing analytics components
- New feature pages or layouts beyond login + shell
- Illustration or icon library creation (use Lucide)

---

## Decisions Log

| Date | Decision | Context |
|------|----------|---------|
| 2026-02-28 | Aesthetic: Esports-forward | Discovery interview |
| 2026-02-28 | Reference: Sports analytics (Opta/StatHead/Wyscout) | Discovery interview |
| 2026-02-28 | Brand feelings: Credible, Precise, Distinctive | Discovery interview |
| 2026-02-28 | Name concept: "Perfect the fundamentals" | Discovery interview |
| 2026-02-28 | Brand concept name: "Iron Precision" | Synthesized |
| 2026-02-28 | Dark mode default; light mode user-persisted; `prefers-color-scheme` as OS initial | User requirement |
| 2026-02-28 | Responsive: desktop/tablet primary, mobile fallback | User requirement |
| 2026-02-28 | Color preferences: purple (primary/premium) and green (secondary/positive) | User requirement |
| 2026-02-28 | Full design system: surfaces, WCAG, focus states, motion, elevation, radii, icons, z-index, states, skeletons, empty/error states | Requirements session |
| 2026-02-28 | Lane Cards: gradient data component; amber↔sapphire gradient encodes lane pressure; scrim layer for text contrast | User requirement |
| TBD | Font pairing selection | Step 1 |
| TBD | Primary color selection | Step 1 |

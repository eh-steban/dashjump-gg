# Frontend Design Tooling Discovery

> **File location:** `private/plans/discovery/`

## Context

We're preparing for coaching interviews and need rapid UI iteration. We evaluated three approaches to accelerate frontend work: AI design-to-code generation (Google Stitch, then alternatives), `@deadlock-api/ui` (pre-built Deadlock game components), and direct agent generation from brand specs. The goal is to find the fastest path from design intent to production-quality React/Tailwind code that matches our brand.

**Goals:** What decisions will this discovery inform?

- Which design-to-code approach produces adoptable output vs visual reference only -- Stitch, v0, Lovable, or direct agent generation
- Which `@deadlock-api/ui` components to adopt vs build ourselves -- affects build-vs-buy decisions for F3 (coaching plans UI), F8 (hero archetypes), and match analysis visualization work
- Whether our brand tokens (`design.md`, future `--dj-*` variables) can drive the chosen tools -- determines if we maintain one design system or juggle overrides
- How to sequence design tooling and component adoption against Phase 1 roadmap priorities

---

## Open Questions

1. **Does Stitch export usable JSX/Tailwind code, or only images/screenshots?** Generate the Login screen and a MatchAnalysis variant using `design.md`. Inspect the raw output format.

2. **Can Stitch-generated code reference CSS custom properties (e.g., `var(--dj-primary)`) when instructed in `design.md`?** Add a `--dj-primary` declaration to design.md, regenerate a button, inspect whether generated code uses `var()` or resolves to hardcoded hex.

3. **Does `@deadlock-api/ui-react` install and render correctly in our Vite/React 18 setup?** `npm install @deadlock-api/ui-react`, render `<DlProvider>` + `<DlItemCard>` in a test page, check for build errors, runtime errors, and visual output.

4. **Can `@deadlock-api/ui` components be styled to match our brand tokens, or do they enforce their own look?** Inspect whether `dl-provider` or individual components accept CSS custom properties, class overrides, or slot-based theming. Check if their visual style (colors, fonts, radius) conflicts with our brand spec.

5. **What data do `@deadlock-api/ui` components fetch on their own vs accept as props?** `dl-item-card` fetches from Deadlock API -- does this duplicate data we'll have locally? Can we pass our own data to avoid redundant network requests? Critical for `dl-hero-minimap-icon` (we already have hero image URLs from `deadlock-api.com/v2/heroes`).

6. **Which current and planned features overlap with `@deadlock-api/ui` components?** Cross-reference the component inventory against our three routes and the roadmap (F1-F14). See the overlap mapping below as a starting point.

7. **If Stitch disappears, what's the migration cost?** For each route where Stitch output was adopted: would replacing it require rewriting structure + logic, or just swapping className strings?

---

### Component-to-Feature Overlap Mapping (starting point for Q6)

| Component | Status | Current route overlap | Roadmap feature overlap |
|-----------|--------|-----------------------|-------------------------|
| `dl-hero-minimap-icon` | WIP | MatchAnalysis minimap (currently custom hero dots) | F8 heatmaps |
| `dl-hero-card` | WIP | ProfilePage hero display, MatchAnalysis PlayerCards | F8 hero archetypes, F3 student file |
| `dl-item-card` | Ready | None currently | Item purchases (not yet parsed), build analysis |
| `dl-shop-panel` | Ready | None currently | Item build reference during coaching review |
| `dl-build-panel` | WIP | None currently | Build comparison (natural extension of item tracking) |
| `dl-ability-order` | WIP | None currently | Ability usage (not parsed, future) |
| `dl-provider` | Ready | N/A (wrapper) | Required if adopting any component above |

**Note:** WIP components may not be usable yet. Q3 must test what actually renders.

---

## Assumptions

### To Validate

- [ ] Stitch generates valid HTML/JSX, not just images. -- *How to check: generate the Login screen from design.md, inspect whether output is copyable code or a rendered screenshot*

- [ ] `@deadlock-api/ui-react` works with Vite + React 18 without config changes. -- *How to check: `npm install @deadlock-api/ui-react` in the frontend, render a `<DlItemCard class-name="upgrade_clip_size" />`, check for build and runtime errors*

- [ ] `@deadlock-api/ui` components can be visually aligned with our brand tokens (surface colors, border radius, fonts). -- *How to check: wrap a `<DlItemCard>` in a card with our brand styles, inspect whether the component's internal styling clashes or can be overridden*

- [ ] Stitch can generate code using CSS `var()` references when instructed in design.md. -- *How to check: add `--dj-primary: #3A6490` to design.md, instruct "use var(--dj-primary) for interactive elements", inspect output*

### Accepted (not tested here)

- Our Tailwind config (v3/v4) recognizes standard utility classes without plugin changes. *Risk if wrong: generated Tailwind classes may require config additions.*
- `design.md` adequately describes the brand system in a format Stitch can parse. *Risk if wrong: Stitch output won't match brand spec; need design.md revisions first.*
- `@deadlock-api/ui` will continue to be maintained by the deadlock-api org. *Risk if wrong: we'd own a dead dependency. Mitigation: web components are self-contained, can be forked or replaced.*
- The React wrapper package (`@deadlock-api/ui-react`) is up to date with the core StencilJS components. *Risk if wrong: React wrapper may lag behind or have missing component bindings.*

---

## Agent Assignments

| Question(s) | Agent | Approach |
|-------------|-------|----------|
| 1, 2, 7 | frontend-react | Generate Stitch output, inspect format and CSS var handling |
| 3, 4, 5 | frontend-react | Install `@deadlock-api/ui-react`, render components, inspect data flow and styling |
| 6 | frontend-react | Cross-reference component inventory with routes (`Login.tsx`, `ProfilePage.tsx`, `MatchAnalysis.tsx`) and roadmap features (F1-F14 in `private/product/strategy/roadmap.md`) |

**Shard boundary:** If this discovery grows, Q1-2+7 (Stitch) and Q3-5 (deadlock-api/ui) can be split into parallel shards. Q6 depends on Q3 findings (can't map overlap for components that don't render).

---

## Research Standards

Follow `.claude/rules/shared/research.md` for confidence labels, citation format, and scope discipline.

---

## Investigation Approach

### Phase 1 -- Stitch output format (gates all Stitch questions)

Generate the Login screen and one MatchAnalysis variant using `design.md` pasted into Stitch's design system field. Capture the raw output. If Stitch produces images only, Q2 and Q7 are N/A -- document this and move to Phase 2.

**Source:** https://stitch.withgoogle.com/ with `private/brand/stitch/design.md` as input and prompts from `private/brand/stitch/README.md`.

### Phase 2 -- deadlock-api/ui installation and rendering (gates Q4-6)

Install `@deadlock-api/ui-react` in the frontend. Render these components in a throwaway test page:
- `<DlProvider>` wrapping `<DlItemCard class-name="upgrade_clip_size" />`
- `<DlShopPanel active-tab="weapon" />`
- `<DlHeroMinimapIcon>` (WIP -- may not render; document what happens)

Check: build errors, runtime console errors, visual output, network requests (what does it fetch?).

**Source:** https://ui.deadlock-api.com/docs/getting-started for installation. Package: `@deadlock-api/ui-react`.

### Phase 3 -- Feature overlap and brand compatibility

With Phase 2 results in hand:
- Map each renderable component against our three routes and F1-F14
- Test CSS override capability: can we change colors/fonts/radius on the rendered components?
- Check `dl-item-card` and `dl-hero-minimap-icon` data props -- can we provide our own hero/item data instead of letting them fetch?

### Phase 4 -- CSS variable test (if Phase 1 positive)

Re-generate a Stitch button with `var(--dj-primary)` instruction. Inspect output. This determines whether Stitch-generated code can use our future token namespace.

---

## Decision tree (fill in during planning, not after)

| Stitch produces usable code? | UI components integrate? | Path forward |
|------------------------------|--------------------------|--------------|
| **No (confirmed -- MD3 overwrite)** | **Yes (confirmed)** | **Confirmed path.** Stitch is dead for us. Adopt `DlItemCard`, `DlShopPanel`, `DlItemTooltip` for item-related features when item data lands. For layout design, evaluate v0 vs direct agent generation (see tool comparison below). |

### Design-to-Code Tool Comparison

Stitch failed. These are the remaining options for producing designs that frontend agents can reference during builds.

| Tool | What it produces | Brand fidelity | Agent-readable? | Cost | Verdict |
|------|-----------------|----------------|-----------------|------|---------|
| **Google Stitch** | Tailwind + HTML | Bad -- injects MD3 color system, overrides fonts, wrong radius | Yes (code) but requires full restyle | Free | **Rejected.** |
| **v0 (Vercel)** | React + Tailwind + shadcn/ui | Unknown -- supports custom design registries, accepts Figma/screenshots | Yes (React code) | Free tier available | **Test next.** Generates code in our exact stack. Custom registry could reference `design.md` tokens. |
| **Lovable** | Full React apps | Has `.lovable/` design system folder with rules for colors/typography | Yes (React code via GitHub sync) | Paid | **Overkill.** Full app builder when we only need component/layout reference. |
| **Figma** | Visual designs + Dev Mode specs | Full control -- we define every token | Agents read screenshots + `design.md` | Free tier (Dev Mode paid) | **Viable.** Manual design work, but agents already handle code generation well from visual + spec input. |
| **Direct agent generation** | React + Tailwind code | Perfect -- agent reads `design.md` directly | N/A -- agent IS the output | Free | **Baseline.** No middleman. Agent reads brand spec + wireframe sketch, produces code. Test against v0 to see if a middleman adds value. |

**Next step:** Generate the Login screen via v0 AND via direct `frontend-react` agent from `design.md`. Compare output quality to determine whether v0 adds value over the direct path.

### v0 Test Results (2026-04-13)

Same Login prompt as the agent baseline. Output: 7 files including `app/layout.tsx`, `app/page.tsx`, `app/globals.css`, `components/ui/button.tsx`, `components/theme-provider.tsx`.

**Brand fidelity (good):** Every explicit token from the prompt appeared correctly in `app/page.tsx`:

| Criterion | Spec | v0 | Match |
|-----------|------|----|----|
| Page bg `#0c0c10` | inline | `bg-[#0c0c10]` | Yes |
| Card bg `#13131a` | inline style | exact | Yes |
| Card border `rgba(255,255,255,0.09)` | inline style | exact | Yes |
| Card radius 8px | `rounded-lg` | correct | Yes |
| Button bg `#3A6490` | `bg-[#3A6490]` | exact | Yes |
| Wordmark: Exo 2 800, `-0.02em` | `next/font` + `tracking-[-0.02em]` | exact | Yes |
| Tagline: DM Sans 400, `rgba(255,255,255,0.45)` | `font-sans` + `text-white/45` | exact | Yes |
| No shadows | — | none | Yes |

**Scaffolding bloat (bad):** v0 also generated ~300 lines of Next.js + shadcn scaffolding we'd strip:

- `globals.css` -- ~150 lines of `oklch()` CSS variables for a default shadcn theme (`--primary: oklch(0.205 0 0)` etc.) that **don't match our brand tokens at all**. Would conflict with `design.md` if adopted.
- `layout.tsx` -- Next.js `app/` router layout using `next/font/google`, `@vercel/analytics`, irrelevant to our Vite stack
- `components/ui/button.tsx` -- shadcn Button with `class-variance-authority` + `@radix-ui/react-slot`, unused in the generated page
- `components/theme-provider.tsx` -- `next-themes` provider for dark/light switching, we're dark-only
- `styles/globals.css` -- duplicate of `app/globals.css`

**Useful signal-to-noise:** ~40 lines of clean Login code inside ~300 lines of Next.js + shadcn boilerplate.

**Verdict vs agent baseline:** The agent baseline (50 lines, every value token-mapped, zero dependencies, comments audit every choice) is cleaner and more adoptable than v0 output in our current Vite stack. v0 adds no design or layout insight the agent didn't produce on its own. **For Vite + React, direct agent generation wins.**

**Open question: would v0 become worth it if we moved to Next.js?** See "Next.js Consideration" section below.

---

## Probe / Query Artifacts

- `dl-ui-probe.mjs` -- `node dl-ui-probe.mjs` (requires Vite dev server on port 5199) -- screenshot at `/tmp/dl-ui-probe/full-page.png`; console/network output in stdout. Probe deleted after use; recreate from discovery notes if needed.
- CSS variable inspection -- read `node_modules/@deadlock-api/ui-core/dist/main/main.css` for `:root` custom properties; read `dist/collection/components/*/` for per-component styles

---

## Discovery Checkpoint

**Status:** `[x] Complete · Stitch rejected · v0 tested (partial fit) · Direct agent generation recommended · Next.js consideration opened`

### Results

- [x] Q1: Stitch output format -- **confirmed: produces code, but not adoptable** (`code-run`) -- Stitch generates Tailwind config + HTML. However, it injects Material Design 3's entire color system (~35 tokens like `primary-container`, `on-primary-fixed-variant`, `surface-container-low`) on top of the brand input. Our `#3A6490` was demoted to `primary-container` while Stitch invented `primary: #a1cafb`. It also overrode fonts (added Space Grotesk + Inter as defaults, sidelining Exo 2/DM Sans to `font-exo`/`font-dm`) and used wrong border-radius values (0.125rem default instead of 8px). **Verdict: Stitch produces code, but the styling layer is a full MD3 rewrite -- not usable without stripping and replacing the entire Tailwind config.**
- [x] Q2: CSS var support in Stitch -- **invalidated** (`code-run`) -- Stitch resolves everything to hardcoded hex in its generated Tailwind config. It does not emit `var()` references regardless of instruction. No CSS custom property support.
- [x] Q3: ui-react installs and renders -- **confirmed** (`code-run`) -- `@deadlock-api/ui-react@1.2.0` installed cleanly in Vite 7 + React 19 (peer deps: `^18 || ^19`). Zero build errors, zero console errors. All 3 ready components rendered: `DlItemCard` (105 instances in shop panels), `DlShopPanel` (2 instances, weapon + spirit tabs), `DlProvider` (2 wrappers). Shadow DOM hydrated with images from `assets.deadlock-api.com`. Screenshot captured.
- [x] Q4: Brand styling compatibility -- **partially possible** (`code-run`) -- Components expose CSS custom properties on `:root` via `main.css`: `--dl-bg-primary`, `--dl-bg-secondary`, `--dl-text-primary`, `--dl-text-secondary`, `--dl-text-muted`, `--dl-font-family`, `--dl-radius`, `--dl-transition`. These are overridable. **However:** tier badge colors (`#cc8932`, `#6dc04b`, `#c878f0`), name label colors (`#1a1510`), active/imbue tag colors, and the proprietary "Retail Demo" font are hardcoded in component CSS -- not overridable via vars. The game-accurate look is intentional and changing it would mean forking the components. **Verdict: surface colors (bg, text) can align with our brand; game-specific colors (item tiers, categories) cannot and should not be overridden -- they match Deadlock's in-game shop.**
- [x] Q5: Data flow -- **self-fetch with limited override** (`code-run` + `doc-read`) -- Components fetch from `assets.deadlock-api.com/v2/items?language=english` and `assets.deadlock-api.com/v2/generic-data` on mount. `DlItemCard` accepts optional `component-items-data` prop for pre-resolved tooltip data, but the card itself always fetches by `class-name` or `item-id`. **Network impact:** a single page render triggered 130+ requests (2 API calls + 128 asset fetches for images, fonts, icons). Assets are CDN-served and cacheable, but this is significant for pages that also load our own match data. We cannot pass our own hero/item image URLs -- the component resolves them internally.
- [x] Q6: Feature overlap mapping -- **confirmed** (`doc-read` + `code-run`) -- see updated table below
- [x] Q7: Stitch migration cost -- **moot** -- Stitch output requires full Tailwind config replacement before adoption. The HTML structure (divs, layout) could be referenced as a wireframe, but the styling is unusable. Migration cost of Stitch code = cost of rewriting all styles, which is the same as building from scratch. **No adoption recommended.**

#### Updated Component-to-Feature Overlap (Q6)

| Component | React Wrapper? | Status | Current route overlap | Roadmap overlap | Adoption recommendation |
|-----------|---------------|--------|-----------------------|-----------------|------------------------|
| `DlItemCard` | Yes | Ready | None | Item purchases (not yet parsed), build analysis | **Adopt when item data lands.** Renders game-accurate cards with tooltips. Saves us from maintaining item image/tooltip logic. |
| `DlShopPanel` | Yes | Ready | None | Item build reference during coaching review | **Adopt as reference panel.** Full shop layout matching in-game. Useful for coach-student item discussion. Fixed 1080px width may need responsive wrapper. |
| `DlItemGrid` | Yes | Ready | None | Custom item grid layouts | **Consider.** Simpler than ShopPanel, useful if we want item grids outside the shop context. |
| `DlItemTooltip` | Yes | Ready | None | Any item hover context | **Adopt.** Standalone tooltip reusable anywhere we show an item reference. |
| `DlProvider` | Yes | Ready | N/A (wrapper) | Required for above | **Required.** Lightweight config wrapper (tooltips, language, tier badges). |
| `DlHeroMinimapIcon` | **No** | WIP | MatchAnalysis minimap (custom hero dots) | F8 heatmaps | **Skip.** Not in React wrapper. No docs, no props, not renderable. We already have working custom hero dots. |
| `DlHeroCard` | **No** | WIP | ProfilePage, MatchAnalysis PlayerCards | F8 hero archetypes, F3 student file | **Skip.** Not in React wrapper. No docs. |
| `DlBuildPanel` | **No** | WIP | None | Build comparison | **Monitor.** Would be high-value when ready, but not usable now. |
| `DlAbilityOrder` | **No** | WIP | None | Ability usage (future) | **Monitor.** Not parsed, not renderable. |

### Assumptions check

- [x] Stitch generates HTML/JSX -- **held but useless** -- generates Tailwind + HTML, but injects MD3 color system that overwrites brand tokens. Code structure is referenceable; styling is not.
- [x] ui-react works with Vite + React 18 -- **held (and exceeded)** -- works with React 19 + Vite 7. Note: TypeScript 4.9.5 type-checking fails on StencilJS types (uses newer TS syntax), but Vite builds fine since esbuild handles transpilation. Upgrading to TS 5.x would fix type-checking.
- [x] UI components can be brand-styled -- **partially held** -- surface colors (bg, text, radius) are CSS var overridable; game-specific colors (tiers, categories) are hardcoded intentionally
- [x] Stitch can use CSS var() references -- **invalidated** -- resolves to hardcoded hex in Tailwind config, ignores var() instructions
- Accepted assumptions worth flagging: "React wrapper is up to date with core" -- **confirmed for ready components**, all 5 exported match their StencilJS core. WIP components have no React bindings at all, making the "lag" concern moot -- they're simply absent.

### Evidence

```
# Stitch output (Login screen) -- injected MD3 color system
tailwind.config.theme.extend.colors = {
  "primary-container": "#3a6490",     // our #3A6490 demoted to "container"
  "primary": "#a1cafb",              // Stitch invented this
  "surface": "#131317",              // close to our #13131a but not exact
  "surface-container-low": "#1b1b1f", // MD3 token, not in our spec
  "on-primary-fixed-variant": "#1b4973", // MD3 token
  // ... ~30 more MD3 tokens not in our brand spec
}
fontFamily: {
  "headline": ["Space Grotesk"],      // not our font
  "body": ["Inter"],                  // not our font
  "exo": ["Exo 2"],                   // our font, but sidelined
  "dm": ["DM Sans"],                  // our font, but sidelined
}
borderRadius: { DEFAULT: "0.125rem" } // 2px, not our 8px

# Package compatibility
peerDependencies = { react: '^18 || ^19', 'react-dom': '^18 || ^19' }
version = '1.2.0'

# React wrapper exports (only 5 components)
export declare const DlItemCard: StencilReactComponent<...>;
export declare const DlItemGrid: StencilReactComponent<...>;
export declare const DlItemTooltip: StencilReactComponent<...>;
export declare const DlProvider: StencilReactComponent<...>;
export declare const DlShopPanel: StencilReactComponent<...>;

# CSS custom properties (overridable)
:root {
  --dl-bg-primary: #1a1a2e;
  --dl-bg-secondary: #16213e;
  --dl-bg-tooltip: #0f0f1a;
  --dl-text-primary: #e0e0e0;
  --dl-text-secondary: #a0a0b0;
  --dl-text-muted: #6a6a7a;
  --dl-font-family: 'Retail Demo', sans-serif;
  --dl-radius: 4px;
  --dl-transition: 150ms ease;
}

# Hardcoded (not overridable) -- dl-item-card.css excerpts
.tier-badge.weapon { background-color: #cc8932; }
.tier-badge.vitality { background-color: #6dc04b; }
.tier-badge.spirit { background-color: #c878f0; }
.mod-name { font-family: 'Retail Demo'; color: #1a1510; }

# Runtime render check
dl-item-card elements: 105
dl-shop-panel elements: 2
dl-provider elements: 2
Console errors: 0
External network requests: 130+ (2 API + 128 assets)

# TypeScript issue (non-blocking)
node_modules/@deadlock-api/ui-core/dist/types/stencil-public-runtime.d.ts(467,31):
  error TS1139: Type parameter declaration expected.
  (StencilJS types require TS 5.x; Vite build uses esbuild, unaffected)
```

### Deferred questions

All deferred questions from the original scope are now resolved. New questions opened by findings:

1. **Would Next.js migration change the v0 calculus?** See "Next.js Consideration" below. Needs its own spike, not resolvable within this discovery.
2. **Figma workflow viability** -- not tested. Would only become relevant if the agent baseline approach fails for more complex layouts. (Low risk -- agents already read images and specs.) Defer until a specific screen fails the direct agent path.

---

## Next.js Consideration (opened by v0 findings)

> This section opens a new question, not resolved by this discovery. It belongs in a follow-up spike if pursued.

### The question

v0's output was built for Next.js from the ground up: `app/layout.tsx`, `next/font/google`, `next/link`, `@vercel/analytics/next`, shadcn `components/ui/`. On our Vite + React stack, ~80% of its output is irrelevant scaffolding. If we were on Next.js, would v0 become "definitely worth it" instead of "maybe"?

### What would change on Next.js

**Things that become useful:**

- `app/layout.tsx` structure is standard Next.js convention, not boilerplate
- `next/font/google` is the idiomatic way to load fonts in Next.js -- our `<link>` + CSS approach would be replaced with this regardless
- `next/link` is the standard routing primitive
- shadcn/ui + Radix is the defacto Next.js component stack. v0 generates it natively; we'd have a matching library out of the box
- GitHub sync for `v0 → repo` becomes meaningful because v0 and Next.js share conventions
- Vercel deployment becomes plausible -- v0 and Next.js are both Vercel products, the adoption path is paved

**Things that stay wrong even on Next.js:**

- The `oklch()` CSS variables in `globals.css` are a default shadcn theme, not our brand. Still needs full replacement with our brand tokens.
- `components/theme-provider.tsx` with `next-themes` for light/dark switching -- we're dark-only
- v0 still duplicates fonts in `globals.css` AND `layout.tsx`
- shadcn Button variants (`destructive`, `outline`, `ghost`, etc.) include shadcn's design opinions for focus rings, disabled states, etc. that may not match our spec

### Does it change the decision?

**No -- it moves v0 from "not worth it" to "maybe worth it."** The bloat ratio improves from ~80% to perhaps ~40%. The useful parts (layout, font loading, component scaffolding) become useful instead of irrelevant. But the brand layer (`globals.css` oklch theme) is still wrong and would still need to be stripped and replaced.

**The honest answer:** adopting Next.js to justify v0 is backwards. Next.js is a **large architectural change** with implications far beyond design tooling:

| Area | Current (Vite + React 19) | Next.js |
|------|---------------------------|---------|
| Routing | `react-router-dom` v7 | App Router (server + client components) |
| Data fetching | Custom hooks + REST | Server components, server actions |
| Build | Vite (fast, simple) | Turbopack/Webpack (different DX) |
| Deployment | Static + our backend | Optimized for Vercel, runnable elsewhere |
| Server rendering | None | SSR/SSG/ISR available |
| Bundle size | Smaller by default | Framework overhead |
| Learning curve | Familiar React | Adds server components mental model |

**Potential upsides of Next.js beyond v0:**

- SSR for coach-facing pages might help initial-load feel on MatchAnalysis (lots of data)
- Server components could move some data processing off the client
- Incremental Static Regeneration for published coaching reports (F3)
- Vercel's preview deployments for PR review
- Better SEO if we ever need public-facing pages

**Potential downsides:**

- Migration cost: every route, every data hook, every component needs review
- Our current Vite stack is **already excellent**: React 19, Tailwind v4, Vite 7 -- very modern, very fast
- Next.js `app/` router is still a relatively new paradigm with evolving best practices
- Adds vendor lean toward Vercel
- `@deadlock-api/ui-react` components use StencilJS web components and require `'use client'` wrappers in Next.js App Router -- extra friction

### Recommendation

**Do not migrate to Next.js solely to make v0 more adoptable.** The tooling tail would be wagging the architecture dog.

However, Next.js **might** be worth considering on its own merits (SSR for MatchAnalysis, coaching report publishing for F3, Vercel preview deploys). If that question comes up independently -- separate from design tooling -- spike it then, and v0 becomes a free secondary benefit.

### Which product surfaces actually want Next.js?

Re-examined against `private/product/strategy/` on 2026-04-13. The framing shift: don't think "migrate the app" -- think "which surfaces are Next.js-shaped and can live at their own deployment."

**Strong fit -- build-in-Next.js candidates:**

- **Shareable coaching reports (F11 output viewed through F3 student file).** Best case by far. A coach runs an LLM match summary, attaches it to a student's profile, and the student opens the link on Discord or their phone. First-paint matters (student opens it once, maybe twice -- React Query's cache never warms). OG images matter (Discord link previews drive open rate). Printable HTML beats SPA dashboards when coaches want "send as PDF." ISR fits (report generated once per session, immutable after). Roadmap weight: F11 collapses coach prep time, Phase 2-3 priority.
- **F14 [redacted event] player accounts (when triggered).** Forced activation event -- 300 players land on their own profile simultaneously. Fast initial paint at scale matters. Gated behind auth so no SEO win, but SSR still helps on cold load. Build trigger is "[redacted event] date confirmed" per F13 -- not an architecture decision to make today.

**Weak fit -- earlier candidates reconsidered:**

- **F12 Patch contextualizer.** Reread the spec -- it's an *in-app coaching alert* for coaches watching their roster, not a public-facing changelog. "Player X mains Hero Y, watch their farm efficiency." Authed, personalized, live data. SPA wins. A separate public "Patch 4.2 impact analysis" page would be a *different* feature the roadmap doesn't list.
- **Organic search content pages.** Vision Bet #6 explicitly chooses professional-first launch over broad casual appeal. GTM Phase 1-2 is warm intros via [redacted coach] and [redacted coach]'s referral network -- "small and insular." SEO-driven content is a Phase 4-5 concern after coach validation. Don't let a deferred maybe drive architecture now.

**Poor fit -- keep on SPA:**

- **MatchAnalysis** (already identified by owner -- specific audience, live data, not content)
- **Coach dashboards / student file editing** (authed, live, workflow-heavy)
- **F13 [redacted event] triage dashboard** (coach workflow, live data, filtering/sorting)
- **Parser/admin tooling**

### Revised recommendation

Don't migrate. When F11 is ready to ship, spike a standalone Next.js report viewer at `reports.dashjump.gg` (or equivalent subdomain/route prefix) that hits the existing FastAPI backend. The SPA at the main app surface stays on Vite + React 19. This is the pattern Linear, Vercel, Cal.com and others use -- marketing/public surfaces on Next.js, product app on whatever fits the workflow. It also contains v0's usefulness to the surfaces that actually benefit from it (reports, future marketing) without forcing the coach workflow to absorb the framework change.

Marketing site (`dashjump.gg` landing) can adopt the same pattern independently at any time -- it's cheap, isolated, and benefits from SSR on its own merits.

**If considered later, the spike should answer:**

1. What breaks when moving from `react-router-dom` to Next.js App Router? (Each of our 3 routes: Login, Profile, MatchAnalysis)
2. Do `@deadlock-api/ui-react` components work inside Server Components or require `'use client'` everywhere?
3. Does SSR meaningfully improve MatchAnalysis initial-paint? (Most match data is fetched from our API which already returns JSON -- SSR may add little)
4. Migration cost for the 364-line `MatchAnalysis.tsx` with its Minimap, PlayerCards, coordinate transforms, and echarts integration
5. Deployment model changes: can we still run alongside our Python backend, or does this push us toward Vercel?

### Artifact for follow-up

- `v0 vs agent baseline comparison` -- this discovery; serves as the tooling motivation for any future Next.js spike
- `agent-baseline.tsx` -- proof that high-quality design output is achievable from `design.md` + agent alone, independent of framework choice

---

**STOP. Present the following to the user before doing anything else:**

1. Answers to every open question with confidence labels
2. Data model or approach recommendation based on findings
3. Enrichment opportunities worth flagging for later
4. Unresolved questions and what would resolve them

Await user decision. If approved, create an implementation plan before writing any code.

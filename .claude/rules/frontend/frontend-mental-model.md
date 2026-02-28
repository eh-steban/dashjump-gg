# Frontend Mental Model

## Status: TODO

This file is a stub. Populate it as architectural patterns and constraints are discovered in the frontend service.

## What Goes Here

Service-specific architecture deep-dives that are:
- Non-obvious constraints that would cause expensive debugging if not documented
- Visualization patterns specific to match analytics
- State management patterns for time-scrubbing and game phase filtering
- Performance constraints unique to rendering large datasets

## Candidate Topics (to be expanded)

### Timeline Scrubbing Architecture
The match timeline allows coaches to scrub through game time. When this is built out, document:
- How game_start_marker is used to convert user-selected time to position index
- How the minimap canvas updates on scrub
- How position data is loaded/cached for smooth scrubbing
- See `.claude/rules/parser/parser-mental-model.md` for the conversion formula

**Pre-wire:** Before implementing, load `private/learnings.md#demo-timeline-offset-reconciliation-pattern`.

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

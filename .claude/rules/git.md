# Git Standards

## Commit Messages

### Format

Use a short imperative subject line, optionally followed by a brief body with bullet points for context.

```
Add creep wave tracking to lane pressure analysis

- Parse all four creep entities per wave
- Expose wave data via /parse endpoint
- Store snapshots at 1-second intervals
```

### Rules

- **No co-author attribution** — Never append `Co-Authored-By` or similar lines to commit messages
- **No implementation details** — Describe *what* changed and *why*, not *how* it was implemented
- **Keep it minimal** — Three to four bullet points is enough; don't pad with obvious or redundant points

### What to Write

| Good | Bad |
|------|-----|
| "Add lane pressure visualization" | "Add LanePressureChart component that uses useMemo to memoize the filtered creep wave data array" |
| "Fix creep wave count off-by-one" | "Change `<= 4` to `< 4` in creep entity loop condition" |
| "Expose boss state in parser output" | "Add `boss_snapshots: Vec<BossSnapshot>` field to ParsedMatchResponse struct and serialize with serde" |

### Bullet Point Guidance

- Use bullets only when there are genuinely distinct changes worth calling out
- Aim for three to four bullets maximum
- Each bullet should convey *impact*, not mechanics

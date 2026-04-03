# Research Standards

Guidelines for any agent performing research -- fetching upstream sources, answering domain questions, or reverse engineering undocumented behavior.

Follow the project Writing Style (`.claude/CLAUDE.md`) in all research output.

## Core Rules

- **Say "I don't know" when uncertain** -- an honest gap is more useful than a confident guess. Flag the gap and suggest how to fill it (fetch a file, inspect a field, run a test).
- **Verify with citations** -- every factual claim must name its source (file path, URL, line number, or proto field). Don't assert what a field does without pointing to where that's established.
- **Use direct quotes for factual grounding** -- when a field name, message name, or behavior is drawn from source code or docs, quote it verbatim rather than paraphrasing.

## Confidence Labeling

Tag interpretations explicitly:

| Label | Meaning |
|-------|---------|
| `confirmed` | Directly observed in source (proto file, code, official docs) |
| `inferred` | Strongly implied by field names, types, or neighboring fields |
| `hypothesis` | Plausible guess -- needs validation (replay test, inspector check) |

Example:
```
- `m_iDamage` (int32) -- confirmed: field name matches damage accumulator in CSPlayerPawn
- `m_nFlags` (uint32) -- inferred: adjacent to movement fields, likely encodes state flags
- `m_flUnknown` (float32) -- hypothesis: may be a cooldown timer based on position in message
```

## Citation Format

| Source type | Format |
|-------------|--------|
| Local file | `path/to/file.rs:42` |
| Proto field | `MessageName.field_name` (valveprotos-rs, commit or date) |
| GitHub file | Full raw URL with ref (commit SHA preferred over `main`) |
| Upstream repo | Repo name + path + Last Fetched date |

## Scope Discipline

- Answer the question asked -- don't expand scope to adjacent fields or features unless directly relevant
- If a question can't be answered without fetching live data, say so before fetching
- When a question spans multiple messages, answer each one separately with its own citations

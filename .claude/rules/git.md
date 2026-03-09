# Git Standards

## Commit Messages

### Format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Example:**
```
feat(parser): add creep wave tracking to lane pressure analysis

- Parse all four creep entities per wave
- Expose wave data via /parse endpoint
- Store snapshots at 1-second intervals
```

---

### Types

Commits MUST use one of the following types:

| Type | When to Use |
|------|-------------|
| `feat` | New user-facing feature (SemVer MINOR) |
| `fix` | Bug fix (SemVer PATCH) |
| `refactor` | Code restructuring without behavior change |
| `perf` | Performance improvement |
| `test` | Adding or updating tests |
| `docs` | Documentation only |
| `chore` | Tooling, dependencies, build process |
| `ci` | CI/CD pipeline changes |

---

### Scope

Scope is OPTIONAL. When included, it MUST describe the affected service or area in lowercase, enclosed in parentheses.

```
feat(parser): ...
fix(frontend): ...
chore(backend): ...
```

---

### Breaking Changes

Breaking changes MUST be indicated in one of two ways:

1. Append `!` after the type/scope: `feat!: remove legacy parse endpoint`
2. Include a `BREAKING CHANGE:` footer in the commit body

---

### Rules

- Subject line MUST follow `<type>[optional scope]: <description>`
- Type MUST be one from the types table above
- Description MUST be written in imperative mood (e.g., "add", "fix", "remove")
- Subject line MUST NOT exceed 72 characters total
- Body SHOULD be preceded by a blank line
- Body bullets SHOULD convey impact, not mechanics — three to four bullets is enough
- MUST NOT append `Co-Authored-By` or any attribution lines
- SHOULD NOT include implementation details — describe *what* changed and *why*, not *how*

---

### What to Write

| Good | Bad |
|------|-----|
| `feat: add lane pressure visualization` | `feat: add LanePressureChart component that uses useMemo to memoize filtered array` |
| `fix: correct creep wave count off-by-one` | `fix: change <= 4 to < 4 in creep entity loop condition` |
| `feat(parser): expose boss state in output` | `feat(parser): add boss_snapshots: Vec<BossSnapshot> field and serialize with serde` |
| `chore: upgrade parser dependencies` | `chore: run cargo update and bump serde from 1.0.195 to 1.0.197` |

Commit staged and unstaged changes following dashjump-gg git conventions.

1. Run `git status` (never -uall) and `git diff` (staged + unstaged) to understand what changed
2. Run `git log --oneline -5` to match recent commit message style
3. Draft a commit message:
   - Imperative subject line, under 70 chars
   - Optional body: 3–4 bullets describing impact/why, not mechanics
   - NO Co-Authored-By or attribution lines (ever)
   - No implementation details — what changed and why, not how
4. Stage relevant files by name — never `git add -A` or `git add .`
   Do not stage .env files, credentials, or large binaries
5. Commit via HEREDOC to preserve formatting
6. Run `git status` to confirm success. Do not push unless asked.

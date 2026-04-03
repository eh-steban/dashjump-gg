Commit staged and unstaged changes following dashjump-gg git conventions.

**Read `.claude/docs/infra/git.md` before drafting the commit message.** That file is the authoritative source for types, formatting rules, and good/bad examples. Follow it exactly.

1. Run `git status` (never -uall) and `git diff` (staged + unstaged) to understand what changed
2. Read `.claude/docs/infra/git.md` for commit message conventions
3. Draft a commit message following git.md exactly
4. Stage relevant files by name — never `git add -A` or `git add .`
   Do not stage .env files, credentials, or large binaries
5. Commit via HEREDOC to preserve formatting
6. Run `git status` to confirm success. Do not push unless asked.

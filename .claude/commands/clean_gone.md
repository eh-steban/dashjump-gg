Clean up local git branches marked as [gone] (deleted on remote but still present locally).

1. Run `git fetch --prune` to update remote tracking info
2. Run `git branch -vv` to identify branches marked [gone]
3. For each [gone] branch:
   - Check for associated worktrees (`git worktree list`) and remove them first if present
   - Delete the branch with `git branch -d` (safe delete); use `-D` only if unmerged but confirmed stale
4. Confirm with `git branch -vv` — no [gone] branches should remain

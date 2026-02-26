Write the current working state to private/CONTEXT.md for machine-switching.

Include:
- Active experiment and current step
- What was just completed
- What's in progress (any uncommitted work?)
- Next planned action
- Any open questions or decisions needed
- Files recently modified
- Any context that would be lost (conversation insights, debugging state)

Format as a brief, scannable document that a fresh Claude session can read
and immediately resume from. Keep under 2,000 tokens.

After writing, remind me to commit and push BOTH repos:
  cd private && git add . && git commit -m "context: [brief description]" && git push && cd ..
  git add . && git commit -m "context: [brief description]" && git push

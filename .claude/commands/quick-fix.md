Use the service agent appropriate to the files being changed (backend-python, frontend-react, rust-parser).

Skip the full experiment/spec workflow for small changes (bug fixes, tweaks,
minor improvements).

Use this when:
- Bug fix with clear reproduction
- Style/formatting tweak
- Documentation update
- Dependency update
- Small refactor (< 50 lines changed)

Do NOT use this when:
- Building a new user-facing feature (needs experiment + spec)
- Making architectural changes (needs spec at minimum)
- Any change that affects coach-facing functionality (needs experiment validation)

Process:
1. Describe the change needed
2. Identify affected files
3. Implement directly
4. Run relevant tests
5. Self-review against `.claude/skills/dashjump-compliance/SKILL.md` checklist.
   For changes touching auth or data handling, also invoke the code-reviewer agent.

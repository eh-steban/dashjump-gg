---
name: dashjump-compliance
description: Security requirements, data handling policies, and compliance guidelines for dashJump.gg. Referenced by code-reviewer for security audits and by service agents when handling user data or authentication.
---

# dashJump.gg Security & Compliance

## Compliance Tier Assessment

dashJump.gg is a pre-revenue B2B tool targeting individual esports coaches.
Current compliance posture: Tier 1 (foundational security) + Tier 2 (user
data protection). SOC2 certification is NOT needed at this stage — revisit
only if selling to esports organizations with formal procurement processes.

---

## Tier 1: Application Security (enforce now)

### Authentication & Sessions
- Steam OpenID 2.0: validate return_to URL, verify claimed_id against Steam
- Session tokens: cryptographically random, HttpOnly, Secure, SameSite=Lax
- CSRF protection on all state-changing endpoints
- Session expiration: define max lifetime and idle timeout
- Logout must invalidate server-side session, not just clear cookies

### Input Validation
- All database queries: parameterized (SQLAlchemy models, no string interpolation)
- API input: Pydantic validation on every endpoint
- File uploads (if any): validate type, size limits, no path traversal
- Parser input: treat all replay file data as untrusted

### Secrets Management
- NEVER commit API keys, DB credentials, session secrets, or tokens to git
- Environment variables for local dev
- AWS Secrets Manager or Parameter Store for production (when deployed)
- Rotate secrets if ever exposed

### Transport Security
- HTTPS everywhere in production (no exceptions)
- HSTS header in production
- No mixed content (HTTP resources on HTTPS pages)

### Security Headers (backend must set these)
- Content-Security-Policy: restrict script/style sources
- X-Frame-Options: DENY (prevent clickjacking)
- X-Content-Type-Options: nosniff
- Referrer-Policy: strict-origin-when-cross-origin
- CORS: allowlist specific origins, not wildcard

### Dependency Security
- Run `npm audit`, `pip audit`, `cargo audit` in CI
- Flag and address critical/high vulnerabilities before merge
- Pin dependency versions (lockfiles committed)

---

## Tier 2: Data Protection (enforce before scaling)

### GDPR Applicability
Applies if ANY coaches or their players are in the EU (very likely in esports).
At current scale, compliance is straightforward:

Required:
- Privacy policy: what data collected, why, how long retained, how to delete
- Right to deletion: ability to delete a user's data on request
- Data inventory: document what you store, where, and retention period
- Lawful basis: legitimate interest (analytics service) or consent

Not required at this scale:
- Data Protection Officer
- Data Processing Agreements
- Formal Data Protection Impact Assessment

### Data Handling Rules
- User data: Steam ID, match history, analytics preferences
- Match data: replay files, parsed game events, derived statistics
- Never store data you don't need
- Define retention periods (how long do we keep parsed match data?)

### Steam Terms of Service
- Verify compliance with Steam Web API Terms of Use
- Respect rate limits on Steam API calls
- Don't store or redistribute Steam user data beyond what's needed
- Display required Steam attributions if applicable

### Rate Limiting
- API endpoints: implement rate limiting before public access
- Respect upstream rate limits (Steam API, any third-party services)
- Log rate limit hits for monitoring

---

## Tier 3: Future (when scaling or seeking investment)

These are NOT needed now:
- SOC2 Type II certification (only for enterprise/org sales)
- Formal incident response plan
- Data breach notification procedures
- Comprehensive audit logging
- Backup and disaster recovery strategy
- Penetration testing

---

## For Code Reviewers

**Flag as CRITICAL:**
- Unparameterized database queries
- Missing auth checks on endpoints
- Secrets in code or config files
- Missing CSRF protection on state-changing routes
- User data logged at INFO level or higher

**Flag as WARNING:**
- Missing security headers
- Dependencies with known vulnerabilities
- Missing rate limiting on public endpoints
- Overly permissive CORS configuration

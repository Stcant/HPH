# State — HPH
Updated: 2026-10-25 · Branch: main

## Goal
Provider-neutral authentication and sessions — acceptance: server functions resolve the actor from a verified session through `MembershipRepository::resolve_actor`, never from the demo actor; tenant-scoped SQL uses only session UUIDs.

## Done (latest first, max 5)
- English UI, errors and docs + README recruiter note — merged as `f54564b` (PR #8) — evidence: CI run 38468961276 `checks` and `database` PASS
- Persisted membership lookup `resolve_actor` under RLS (PR #7) — evidence: CI run 38466596064 PASS
- SQLx company module repository, admin-only atomic writes, tested as non-owner role — evidence: CI run 38465298093 PASS
- PostgreSQL schema + RLS tenant isolation — evidence: CI run 38456551456 `database` PASS

## In progress / blockers
- Authentication/sessions — status: not started
- Browser QA (desktop/mobile, hydration, visual) — status: NOT RUN; no browser environment so far

## Known pre-existing failures
- none recorded

## Next (max 3)
1. Design the session model (provider-neutral identity → `UserIdentity` + selected tenant) and agree it before coding.
2. Connect `resolve_actor` and the module repository to server functions behind the session.
3. Run browser QA on desktop and mobile when a build environment is available.

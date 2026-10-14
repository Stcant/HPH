# HPH — Project state

## Active implementation — 2026-10-13

Status: in_progress on `feat/company-workspace`, based on `d6ee299822a8e0efe1f543406a25d317e82afe95`. User authorized all proposed features and PR/merge work. A clean local Git checkout and Rust toolchain are now available.

| Deliverable | Acceptance / verification | Status |
| --- | --- | --- |
| Account login, logout and company selection | Opaque expiring DB sessions; Argon2 passwords; CSRF/origin checks; revoked membership denied; HTTP and DB tests | in_progress |
| Persistent people and time entries | Admin create/edit/deactivate/roles; employee self-clock; idempotent writes; invalid sequences rejected; survives restart | pending |
| Attendance and corrections | Company timezone; daily/monthly closed intervals split at midnight; reason and immutable audit; valid corrected sequence | pending |
| Template customization | Company name, theme, accent and safe local logo; persisted module selection; working Notes module example | pending |
| Browser/portfolio and documentation | Desktop/mobile workflow tests; real screenshot and GIF; English docs; configured CI; PR merge only after gates pass | pending |

Design: keep the existing explicit loopback demo. Add an authenticated server-rendered Rust workspace with standard HTML forms, PostgreSQL transactions and per-operation membership checks. No external identity service, email delivery or deployment is required. All requested scope remains active; this plan does not mark later deliverables complete.

Next action: implement and verify authenticated operational workspace, then complete all rows above. Historical evidence below describes previous increments only.

## Goal and phase

Build a reusable modular business management application with a Rust frontend/backend and a carefully designed interface. The preference for Rust on the frontend supersedes the open frontend choice from the kickoff.

Current increment: English throughout the repository and interface, plus a recruiter Easter egg in the README. Status: **verified** in PR #8 on `chore/english-language`; merge follows the final checkpoint checks. Scope: interface and accessibility text, domain/server errors, English HTML language, unambiguous UTC date display, documentation and existing SSR assertions. Acceptance: English content across maintained files, a readable expandable README note, and passing formatting/SSR/WASM/PostgreSQL checks. No authentication or persistence behavior changes are included.

Previous completed increment: persisted membership SQLx repository, **verified** and integrated into `main` by PR #7, merge `812a5bb21b232039151dd375b4671d019588e284`. CI run [38466818515](https://github.com/Stcant/HPH/actions/runs/38466818515) passed both `checks` and `database`, including tenant-scoped lookup and rejection of missing or revoked memberships under RLS. PR #6 was integrated with merge `231762216f4582e5e7ffe0550497d975f7f153c6`; run [38465298093](https://github.com/Stcant/HPH/actions/runs/38465298093) passed on commit `85573ff444bebe43d7a09da848bd08a80608b601`. Post-merge CI [38467158574](https://github.com/Stcant/HPH/actions/runs/38467158574) passed on `1d58132cba643ba740804275056b6c22fca15050`, the base of this language change. The overall project remains in progress; authentication and browser QA are still missing.

Original checkpoint: 2026-10-12. Current update: 2026-10-13. Public portfolio repository: `Stcant/HPH`. Source files are available as a local snapshot fetched through GitHub; there is no local Git checkout or Rust toolchain. Rust and PostgreSQL verification runs through GitHub Actions.

## Decisions

Leptos + Axum; independent domain core; module catalog and dependencies; responsive CSS and themes; PostgreSQL/SQLx for pooling, migrations and transaction-local tenant context. Application repositories still need to be integrated into the screens. The initial module list is persistent installation configuration validated at startup; interactive demo changes remain temporary. PostgreSQL will persist runtime configuration per organization. The deployment model (dedicated instance or multi-company service) remains open. English is the default language for maintained repository content and the UI. Details and boundaries are in [PROJECT_MAP.md](PROJECT_MAP.md).

## Implemented result

- Dashboard, read-only demo people directory, time tracking, summaries and module configuration.
- essential/team/complete presets and sage/indigo themes in hph.toml; an explicit company module list can persistently override the preset, with dependency and duplicate validation at startup.
- Real server functions with in-memory state: clock in, start break, end break and clock out; sequence and timestamp validation on the server.
- Dependency and administrator checks for activation in the domain; disabling modules preserves events and does not interrupt open work sessions.
- Startup only with HPH_DEMO=true and loopback binding. One synthetic actor and company: no authenticated or persistent operational application yet.
- Startup README, module extension contract and CI workflow.

## Evidence from 2026-10-12

Scope: initial template sources saved with this checkpoint; tests ran before the remote commit. Commit history identifies the files. No browser or database verification is implied by this historical evidence.

| Criterion | Result | Action and evidence |
| --- | --- | --- |
| Module, tenant and time-tracking rules | PASS | cargo test --workspace --features ssr --locked: 6 hph-core tests passed |
| Real RPCs and HTTP rule enforcement | PASS | Same command: 2 Axum router tests passed; duplicate clock-in, disabled module and invalid dependencies rejected |
| Full-page server rendering | PASS | Same command: 1 SSR test passed; company name, demo notice, content and CSS present |
| Frontend and backend build | PASS | cargo-leptos 0.3.11 build: SSR binary, WASM library, CSS and loader generated; exit 0 |
| Formatting | PASS | cargo fmt --all --check: exit 0 |
| Explicit demo startup and local binding | PASS | Running without HPH_DEMO=true and with 0.0.0.0 was rejected in both cases |
| Visual appearance, mobile, navigation and browser hydration | NOT RUN | Browser QA unavailable; build and SSR do not prove interaction or visual quality |
| Authentication, PostgreSQL and multi-company isolation | NOT RUN | Not implemented at that checkpoint; subsequent increments are recorded below |

## Increment: custom module selection — 2026-10-13

Scope: hph.toml `modules` override, preserving presets and validating dependencies/duplicates. Source commit: `8f350db4aec6c1dc3974c6d67086dd9079ace1da`.

| Criterion | Result | Evidence |
| --- | --- | --- |
| TOML accepts a valid company list that overrides the preset | PASS | `explicit_company_modules_override_preset_and_invalid_lists_fail`: TOML deserialized, essential overridden by people/attendance/reports |
| Missing dependencies and duplicate IDs are rejected without changing the selection | PASS | `custom_module_lists_validate_dependencies_and_duplicates` and invalid configuration test |
| Formatting, SSR tests and WASM check | PASS | [Run 38320041661](https://github.com/Stcant/HPH/actions/runs/38320041661), success on `8f350db4aec6c1dc3974c6d67086dd9079ace1da`; 2026-10-12T23:02:44Z–23:03:55Z |

Static selection in hph.toml persists across restarts; changes made in the demo screen remain in memory.

## Increment: identity and membership — 2026-10-13

Scope: domain contracts for user identity and active company membership; the demo server creates the actor from an explicit membership. This increment did not introduce login, sessions or persistence.

| Criterion | Result | Evidence |
| --- | --- | --- |
| One user can have different roles in different companies | PASS | `identity_resolves_only_an_active_membership_in_the_selected_company`: selects the correct Employee membership; rejects inactive membership and a user without membership |
| Demo actor is derived from an explicit membership | PASS | DemoState::new calls UserIdentity::actor_for |
| Formatting, SSR tests and WASM check | PASS | [Run 38454681554](https://github.com/Stcant/HPH/actions/runs/38454681554), all steps successful on `8097d605f6ed95547dec2688cd03ce61c09ee6e3` |

Demo memberships remain in memory. A verified identity must come from authentication middleware, and membership must be read from the database before company operations.

## Increment: multi-company PostgreSQL schema — 2026-10-13

Scope: organizations, external identities, people, memberships, configured modules and idempotent attendance events; company-scoped Row-Level Security. The SQL repository was not yet integrated into the Rust app.

| Criterion | Result | Evidence |
| --- | --- | --- |
| Company data and cross-organization consistency | PASS | Composite foreign keys bind membership/person and attendance/person to the same organization; uniqueness constraints and attendance idempotency key |
| Scoped reads, own memberships and blocked cross-company insert/update/delete | PASS | migrations/tests/tenant_isolation.sql executed against PostgreSQL 18 in CI, run 38456551456 |
| Rust tests, formatting and WASM check | PASS | checks job in run 38456551456 |

The runtime app must set hph.tenant_id and hph.user_id transaction-locally, verify membership and roles in the service, and use a non-owner role without BYPASSRLS. This schema alone does not prove HTTP operation security.

## Increment: PostgreSQL schema and tenant isolation — 2026-10-13

Scope: initial schema for organizations, external identities, people, memberships, company modules and attendance events; forced RLS on company tables; SQL tests with a runtime role without bypass privileges. This increment did not add SQLx repositories to the app.

| Criterion | Result | Evidence |
| --- | --- | --- |
| PostgreSQL migration applies | PASS | database job in CI run 38456551456 |
| Tenant-scoped queries do not expose another company's rows | PASS | Automated tests for visibility, cross-company update/delete and insert, and missing tenant context |
| Rust and WASM checks stay green | PASS | checks job in the same CI run |

## Increment: SQLx application foundation — 2026-10-13

Verified scope: SQLx PostgreSQL pool for the SSR server; separable migrator using a dedicated role; helper that sets user and tenant only for the transaction's lifetime; optional health check. Server functions still use in-memory demo state.

| Criterion | Result | Evidence |
| --- | --- | --- |
| SQLx pool, migration and tenant context compile | PASS | Run 38460315357: cargo fmt, workspace tests and WASM check passed |
| Migration applies twice and PostgreSQL pool is reachable | PASS | database job in run 38460315357; sqlx_pool_migrates_and_scopes_tenant_context |
| Tenant and user context are transaction-local | PASS | Test reads both values inside the transaction and verifies a null tenant after commit |
| Rust, SSR, WASM and existing RLS checks | PASS | checks and database jobs in [run 38460315357](https://github.com/Stcant/HPH/actions/runs/38460315357), commit `6b3964a4acef3b1241ead813f2980c9b9a140cba` |

## Increment: SQLx company module repository — 2026-10-13

Added src/module_repository.rs: tenant-scoped reads requiring active membership; atomic writes of every flag with dependency validation and an admin check in the same transaction. An unsaved selection returns None. UI integration is deferred until the synthetic demo identity is replaced by verified UUIDs from an authenticated session.

| Criterion | Result | Evidence |
| --- | --- | --- |
| Repository scopes queries and writes to the tenant through RLS context | PASS | PostgreSQL test company_module_selection_is_tenant_scoped_and_admin_only passed in the database job of run 38461693770 |
| Only active admin membership can save configuration | PASS | PostgreSQL test verifies admin round-trip and rejection of the employee role |
| Formatting, SSR tests and WASM check | PASS | checks job in run 38461693770 passed |

## Additional verification: non-owner PostgreSQL runtime role — 2026-10-13

The first repository test used a superuser connection, so it did not actually exercise RLS policies. The correction separates the fixture/migration admin pool from the hph_runtime pool, created with NOSUPERUSER NOBYPASSRLS; integration tests exercise repository operations through the runtime role.

| Criterion | Result | Evidence |
| --- | --- | --- |
| Repository tested with effective RLS | PASS | company_module_selection_is_tenant_scoped_and_admin_only ran with NOSUPERUSER NOBYPASSRLS; [run 38465298093](https://github.com/Stcant/HPH/actions/runs/38465298093), tested commit `85573ff444bebe43d7a09da848bd08a80608b601` |
| Formatting, SSR tests and WASM check | PASS | checks job in [run 38465298093](https://github.com/Stcant/HPH/actions/runs/38465298093) passed |

## Increment: persisted membership lookup — 2026-10-13

Scope: MembershipRepository::resolve_actor(user_id, tenant_id) resolves an actor only from an active membership in the selected tenant, inside an RLS-context transaction. UUIDs must come from an authenticated session; login/session and UI integration are excluded.

| Criterion | Result | Evidence |
| --- | --- | --- |
| Active membership returns the correct person, tenant and role for multi-company users | PASS | Dedicated PostgreSQL test with two tenants and a non-owner runtime role |
| Missing membership in a tenant and revoked membership are rejected | PASS | Dedicated integration test; [run 38466596064](https://github.com/Stcant/HPH/actions/runs/38466596064) |
| Formatting, SSR tests, WASM and PostgreSQL checks | PASS | checks and database jobs in [run 38466596064](https://github.com/Stcant/HPH/actions/runs/38466596064), tested commit `a5617a2e8f68f647feb8a1b07fc379d1776292c8` |

## Increment: English language and recruiter Easter egg — 2026-10-13

Source commit: `2a21a36cd0700a0c066a971228508b56758f710b`, PR #8. [CI run 38468961276](https://github.com/Stcant/HPH/actions/runs/38468961276) passed both jobs.

| Criterion | Result | Evidence |
| --- | --- | --- |
| English UI, errors, accessibility and maintained documentation | PASS | Source review; existing SSR page test checks English content and `lang="en"` |
| Formatting, workspace/SSR tests and WASM compilation | PASS | `checks` job in run 38468961276 |
| PostgreSQL migrations, RLS and repository integration | PASS | `database` job in the same run |
| Expandable recruiter note | PASS | README source contains the recruiter side quest and contact link |
| Visual layout and browser interaction | NOT RUN | No browser QA in this increment |

## Relevant resolved problems

- An executor interruption invalidated an initial test run; valid results came from subsequent executions.
- Deeply nested SSR component rendering caused a stack overflow. Component boundaries were converted to AnyView; all 9 tests then passed with the standard stack size.

## Limitations and open work

The demo loses data and interactive settings on restart. Profiles are synthetic and the administrator role is fixed. The domain tenant model does not establish isolation for a real service. No deployment or external exposure has been performed.

Leave, time off, shifts, employee CRUD, corrections/audit, application-level idempotency, exports and contract rules remain to be implemented. The GitHub Template repository flag has not been changed.

## Next action

Merge PR #8 after the final checkpoint checks. The next functional increment remains provider-neutral authentication/sessions and connecting session context to repositories; do not use the demo actor for tenant-scoped SQL. Then implement people and attendance repositories. Run desktop/mobile browser QA when an executable build and browser environment are available. A local source snapshot is available, but Rust verification still runs through GitHub CI.

## Files and reading order

Code: Cargo.toml/Cargo.lock, rust-toolchain.toml, crates/hph-core, src and style. Configuration: hph.toml. Startup and limitations: [README](../README.md). Extensions: [MODULES](../docs/MODULES.md). Keep these two harness files as the canonical state and map.

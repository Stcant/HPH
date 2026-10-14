# HPH — Project map

## Product and constraints

A personal Rust business-management project and reusable company template. Time tracking and attendance are the first operational workflow. Companies select modules and customize their appearance. English is the default language for UI, accessibility, errors, comments and documentation. The public portfolio repository is `Stcant/HPH`; no public application deployment is included.

## Architecture

| Area | Choice and boundary |
| --- | --- |
| Operational UI | Rust-generated HTML with accessible standard forms and CSS; works without browser JavaScript |
| Interactive demo | Existing Leptos 0.8 SSR + WASM hydration, explicitly enabled by HPH_DEMO=true on loopback |
| Server | Axum 0.8 + Tokio; separate routers prevent the demo's synthetic actor reaching persistent routes |
| Shape | Modular monolith with shared framework-independent rules in hph-core |
| Persistence | PostgreSQL + SQLx, versioned additive migrations, forced tenant RLS, restricted runtime role |
| Identity | Argon2id local accounts; 256-bit opaque sessions stored as hashes, 8-hour expiry, logout/password-change revocation |
| Request boundary | Exact Origin on POST; authenticated forms carry session/company-bound CSRF; HttpOnly/Strict cookies, Secure for HTTPS |
| Company operations | One scoped transaction and organization lock; fresh active membership/employee and role checks before work |
| Time | Server UTC timestamps; validated sequence and idempotency; IANA display/report timezone; split closed intervals at local midnight |
| Modules | Typed catalog and dependencies; company-persisted flags; disabling preserves data and blocks protected operations |
| Design | Responsive sidebar/cards/tables/forms; sage/indigo, company accent and local asset logo; system fonts |

The operational UI intentionally starts with server-rendered forms so every protected operation has an explicit HTTP boundary and remains functional without hydration. It does not replace the interactive Leptos demo. A future interactive operational frontend can reuse the same service contracts. Rust application code requires no Node; Playwright's Node dependency is for browser QA only.

## Files and contracts

- `src/main.rs`: chooses explicit demo mode or authenticated operational mode; `--migrate` and `--bootstrap` are operator-only CLI paths.
- `src/auth.rs`: Argon2 hashing/verification, login lockout, session token generation/resolution, company/account bootstrap. Bootstrap verifies the password before adding a company to an existing account.
- `src/workspace_web.rs`: protected routes, same-origin/CSRF checks, cookie handling, bounded IP login attempts and hash concurrency, response headers.
- `src/operations.rs`: transaction-scoped company selection, employee edits, idempotent clocking, reasoned timestamp corrections, module/settings updates, Notes and local-calendar summaries.
- `src/workspace_ui.rs`, `style/workspace.css`: operational Rust views and layout. Escape all dynamic text/attributes. Local logos are deployment-owned assets under `/assets/`.
- `src/database.rs`: bounded SQLx pool, owner migrator, transaction-local tenant/user settings and runtime-role checks.
- `src/module_repository.rs`, `src/membership_repository.rs`: lower-level building blocks with restricted-role integration tests. The operational service performs related checks together in one transaction rather than composing separate repository transactions.
- `crates/hph-core/src/lib.rs`: typed module IDs/catalog/dependencies, membership/role contracts, attendance state transitions and closed-interval rules.
- `src/app.rs`, `src/api.rs`, `src/server.rs`: separate Leptos demo with synthetic identity and in-memory state. Notes is not offered in this mode.
- `migrations/0001_company_data.sql`: base company schema and RLS. `0002_workspace.sql`: active profiles, branding, credentials, sessions, immutable audit and Notes.
- `compose.yaml`, `scripts/init-runtime.sh`: local PostgreSQL and non-owner runtime-role setup. `hph.toml` configures the demo only.
- `src/workspace_tests.rs`: real operational HTTP/DB integration. `scripts/browser-smoke.mjs`: real server desktop/mobile flows and image captures. `.github/workflows/ci.yml`: Rust/SSR/WASM, PostgreSQL and browser gates.

## Invariants

- User identity comes from an unexpired server-side session, not form fields. Selecting a company requires active membership and active employee state.
- Every company operation rechecks membership inside its transaction. The organization lock serializes reads and writes, including deactivation and module changes. This is a deliberate initial consistency/throughput tradeoff.
- Employees see their own profiles/events and can clock only for themselves. Administrators manage company people, modules, appearance and corrections. Company notes are shared with active members when enabled.
- A person cannot be deactivated during an open shift. An administrator cannot deactivate/demote their own account. Editing a profile email does not alter global login credentials.
- Timestamp corrections require a reason, reject future/colliding/invalid-sequence times, and atomically record old/new values and actor in immutable audit. Event creation is idempotent per company/person/request key.
- Audit rows are append-only through a PostgreSQL trigger. The display shows the latest 50; deletion/retention is not implemented.
- Module dependencies are authoritative in hph-core. Attendance requires People; Reports requires Attendance; Notes requires People. Invalid changes preserve the previous selection.
- Existing applied migrations are immutable. Demo records are never inserted into an operational company by normal startup.

## Remaining boundaries

Accounts are locally managed. Email invitations, password recovery, OIDC/MFA, missed-event insertion/deletion, approval workflows and export are not implemented. Operator setup and credential delivery remain manual. The application has not been publicly deployed or certified for production.

Reads load company event history in memory, and the shared company lock prioritizes correctness over throughput. Pagination, retention, concurrency/load testing, backup recovery, monitoring and deployment-specific security review are needed before a large installation.

Reports exclude open intervals and breaks; they do not calculate payroll, overtime, contractual rounding or leave balances. Views display whole minutes while stored events retain seconds. Leave, shifts, documents, expenses and projects are future modules; payroll, accounting and inventory remain outside this increment.

## Reading order

[State and current evidence](PROJECT_STATE.md) → [README](../README.md) → [Workspace setup](../docs/WORKSPACE.md) → [Database](../docs/DATABASE.md) → [Module contract and Notes walkthrough](../docs/MODULES.md).

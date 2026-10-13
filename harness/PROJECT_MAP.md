# HPH — Project map

## Confirmed requirements

- A personal business management project with time tracking and attendance, extensible to additional functions.
- A reusable template: each installation/company can select its own modules.
- Each company composes HPH around its processes; module selection belongs to the company and must become persistent in the application.
- After initially leaving technology choices open, the owner expressed a preference for Rust throughout. Adopted choice: a Rust frontend and backend.
- Visual quality and a carefully designed user experience are central requirements.
- English is the default language for the interface, accessibility labels, errors, comments and documentation (requested on 2026-10-13). Preserve proper names and stable identifiers.
- Public portfolio repository: https://github.com/Stcant/HPH, branch `main`. No application deployment requested.

## Architectural decisions

| Area | Choice and rationale |
| --- | --- |
| Frontend | Leptos 0.8, SSR + WASM hydration: Rust components and contracts in one application |
| HTTP | Axum 0.8 + Tokio, using the official Leptos integration |
| Organization | Modular monolith: one deployment, independent rules in hph-core, no initial microservices |
| Modules | Typed catalog, declared dependencies and server-enforced activation; persistent per-company configuration still needs application integration |
| Configuration | hph.toml for company, theme, preset or explicit module list; demo runtime changes are temporary |
| Design | CSS tokens, sage and indigo themes, sidebar, cards, tables and responsive layout |
| Persistence | PostgreSQL schema and RLS policies tested in CI; SQLx pool, migrations, module and membership repositories. Screens still use in-memory state; the API awaits an authenticated identity |
| Identity | Rust contracts and a SQLx repository for active membership, role and person per tenant. Login, sessions and connecting the resolver to server functions remain to be implemented |
| Language | English interface and repository content; HTML language is en. UTC timestamps use an unambiguous YYYY-MM-DD display format |

Tradeoff: Rust end to end shares types and expertise, but requires WASM builds and offers a smaller UI ecosystem than React. Leptos generates a JavaScript loader; application code remains Rust. Do not claim that the browser runs no JavaScript at all.

## Implementation

- `crates/hph-core/src/lib.rs`: catalog and dependencies, user identity and company membership, attendance state machine, closed-interval calculations and tests.
- `migrations/0001_company_data.sql`: multi-company PostgreSQL schema and RLS policies; `migrations/tests/tenant_isolation.sql` verifies visibility and cross-company writes.
- `src/database.rs`: SQLx pool, runtime migrator, health check and transactions with transaction-local RLS context.
- `src/module_repository.rs`: atomic company module reads/writes under RLS, active membership and admin role checks. Not connected to server functions until an authenticated session provides verified UUIDs.
- `src/membership_repository.rs`: resolves an actor from a persisted active membership in the selected tenant, given an identity and tenant selection validated by the authentication layer.
- `src/app.rs`: dashboard, demo directory, time tracking, reports, module configuration and shared components.
- `src/api.rs`: snapshot, configuration and time-entry server functions. The acting identity comes from server context.
- `src/server.rs`: configuration, in-memory demo state, Axum router and HTTP/SSR tests; optional PostgreSQL health check.
- `src/main.rs`: startup requires HPH_DEMO=true and a loopback address; runtime pool and separate migrator are optional environment settings.
- `style/main.css`: visual design, tokens and mobile styles; no remote fonts or assets required.
- `Cargo.lock`: resolved versions; `rust-toolchain.toml`: toolchain and target.
- `.github/workflows/ci.yml`: formatting, SSR tests, WASM check and PostgreSQL integration.

## Modules

Initial company configuration can use a preset or an explicit validated list in hph.toml; the UI can change the selection during the demo. Production runtime configuration must be persisted and authorized per organization.

Core is always active. People contains read-only synthetic profiles. Attendance depends on People; Reports depends on Attendance. Disabling modules preserves events; open work sessions prevent Attendance from being disabled. Reports show closed work intervals in the session, excluding breaks.

## Current boundaries

This is a local demo with a synthetic administrator membership. Identity/membership contracts, RLS schema, module repository and persisted membership repository are covered by CI. Login, sessions and connecting repositories to server functions are not integrated. The tenant is modeled and tested in the domain, but this does not prove a secure multi-company service. Data and module changes reset on restart. Refreshing other tabs is manual.

Supporting multiple companies requires verified membership, mandatory tenant scope on queries/mutations, migrations, constraints and database isolation tests. No real data before this phase is complete.

Timestamps are UTC and server-generated; sequences are validated. Company calendars, display time zones, contract rules, rounding, corrections/audit and retry idempotency require dedicated application implementation.

## Customization model

HPH Core remains shared. Modules are versioned Rust features in the template, selectable by company administrators and enforced on the server. Disabling them hides and blocks their operations without deleting data. The demo applies the selection to one in-memory workspace. Persistent configuration must be associated with the organization. The deployment model—dedicated instance or multi-company service—remains open, so the tenant boundary must stay explicit from the start.

## Product evolution

First target operational workflow: authentication, a real employee directory, persistent time entries and manager oversight. Then leave requests and an attendance calendar. Shifts, documents, expenses, projects, customers, invoicing, inventory and integrations remain candidate modules. Payroll and accounting require separate specifications.

## Reading order

- [Project state](PROJECT_STATE.md): evidence and next action.
- [Startup and limitations](../README.md).
- [Module contract](../docs/MODULES.md).

Primary references consulted: https://book.leptos.dev/getting_started/index.html ; https://book.leptos.dev/ssr/21_cargo_leptos.html ; https://github.com/leptos-rs/start-axum ; https://docs.rs/axum/latest/axum/ ; https://www.postgresql.org/docs/current/ddl-rowsecurity.html .

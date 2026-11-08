# HPH

Reusable foundation for modular business management software (time tracking first), Rust end to end; currently a local demo, not for real company data.

## Stack
- Rust stable (edition 2024), `rust-toolchain.toml` adds `wasm32-unknown-unknown`, rustfmt, clippy
- Leptos 0.8 (SSR + WASM hydration), Axum 0.8 + Tokio, cargo-leptos 0.3.11
- PostgreSQL 18 with SQLx 0.9 and Row-Level Security; `Cargo.lock` is committed, use `--locked`
- Cargo workspace: root crate `hph` + `crates/hph-core` (pure domain rules)

## Commands (repository root)
- Run: `HPH_DEMO=true cargo leptos watch` → http://127.0.0.1:3000 (refuses to start without `HPH_DEMO=true` or on a non-loopback address)
- Format: `cargo fmt --all --check`
- Test: `cargo test --workspace --features ssr --locked` (compiles the workspace; slow on a cold `target/`)
- WASM check: `cargo check --target wasm32-unknown-unknown --features hydrate --lib --locked`
- Build: `cargo leptos build`
- PostgreSQL tests run only in CI (`.github/workflows/ci.yml`, job `database`); locally they need `HPH_TEST_DATABASE_URL` and `HPH_TEST_ADMIN_DATABASE_URL`

## Structure
- `crates/hph-core/src/lib.rs` — module catalog and dependencies, roles, memberships, attendance state machine; no Leptos/HTTP/DB code
- `src/api.rs` — server functions; the acting identity always comes from server context, never from the client
- `src/app.rs` — Leptos screens and components; `style/main.css` — tokens, sage/indigo themes
- `src/server.rs` — demo config/state, Axum router, HTTP and SSR tests
- `src/database.rs`, `src/module_repository.rs`, `src/membership_repository.rs` — SQLx pool, migrations, tenant-scoped repositories
- `migrations/` — schema and RLS policies; `migrations/tests/tenant_isolation.sql` checks isolation
- `hph.toml` — organization, theme, preset or explicit `modules` list, validated at startup
- `docs/MODULES.md` — module extension contract; `docs/DATABASE.md` — tenant boundary rules

## Conventions and gotchas
- Interface text, errors, accessibility labels, comments and docs are English.
- Module rules live only in `hph-core` (`ModuleId`, `catalog()`, `ModuleSet`), shared by server and browser; no second catalog in the UI.
- Disabling a module hides and blocks its operations but never deletes its data; open work sessions block disabling Attendance.
- Every company-data DB operation runs in `Database::begin_tenant_transaction(user_id, tenant_id)` with transaction-local settings; the runtime role must not own tables or have `BYPASSRLS`.
- Screens still use in-memory demo state with a synthetic actor; its IDs are not DB UUIDs. Never adapt it into a tenant-scoped SQL identity.
- Deeply nested SSR components overflowed the stack; component boundaries use `AnyView`.
- Timestamps are UTC, server-generated, displayed as `YYYY-MM-DD`.
- The deployment model (dedicated instance or multi-company service) is undecided: keep the tenant boundary explicit everywhere.

## Do not
- Expose the demo through a proxy, tunnel or public network, or load real company data before authentication and persistence are integrated.
- Change the GitHub "Template repository" setting or deploy without an explicit request.

# HPH

**Work, in harmony.** A modular business workspace built in Rust, and a personal project about learning by building.

HPH has two modes: an authenticated PostgreSQL workspace for local evaluation, and a separate in-memory Leptos demo. Company administrators choose their modules, appearance and timezone. Employees get their own account and attendance history.

## What you can do

- Sign in, select a company and sign out with expiring server-side sessions.
- Create, edit and deactivate employees; assign employee or administrator roles.
- Record clock-ins, breaks and clock-outs. Changes survive application restarts.
- Review daily/monthly recorded time in the company timezone, including overnight shifts and daylight saving changes.
- Correct an event timestamp with a reason, retaining before/after values in append-only audit history.
- Customize company name, theme, accent color and a deployment-owned logo.
- Enable People, Attendance, Reports and Team notes independently, respecting dependencies.
- Use Team notes as a small, complete example when building your own module.

The operational interface is responsive, server-rendered Rust with standard HTML forms. It works without JavaScript. The original demo uses Leptos SSR and WebAssembly hydration. There is no separate React/TypeScript application and no Node requirement to run HPH.

## Run the persistent workspace

You need Rust and Docker Compose. PostgreSQL runs locally; no cloud subscription is required. See [the complete setup guide](docs/WORKSPACE.md) for accounts, environment variables, upgrades and the restricted database role.

```bash
git clone https://github.com/Stcant/HPH.git
cd HPH
# Set your own local database passwords; do not commit them.
export HPH_POSTGRES_PASSWORD='replace-with-a-local-owner-password'
export HPH_RUNTIME_PASSWORD='replace-with-a-different-runtime-password'
docker compose up -d
```

Follow [docs/WORKSPACE.md](docs/WORKSPACE.md) to bootstrap your first company and administrator, then run:

```bash
# HPH_DATABASE_URL must use hph_runtime, never the migration owner.
cargo run --features ssr --locked
```

Open **http://127.0.0.1:3000**. Login and company selection lead to the persistent workspace. The server rejects superuser, BYPASSRLS and table-owner runtime connections.

## Try the in-memory demo

No database or account setup is required:

```bash
cargo install cargo-leptos --version 0.3.11 --locked
HPH_DEMO=true cargo leptos watch
```

In PowerShell, set `$env:HPH_DEMO = "true"` first. Open **http://127.0.0.1:3000**. The demo requires loopback binding and uses fictional profiles. It must not be exposed through a public proxy or tunnel. Demo events and module changes are shared between browser tabs and reset when the server restarts.

`hph.toml` configures only this demo. Persistent companies are managed through the authenticated Workspace screen.

## Modules and customization

| Module | Dependency | Behavior |
| --- | --- | --- |
| People | None | Employee profiles, activation and roles |
| Time and attendance | People | Self-service clocking and administrator timestamp corrections |
| Reports | Attendance | Local daily/monthly totals from completed intervals |
| Team notes | People | Shared company noticeboard; operational mode only |

Disabling a module hides and blocks its operations without deleting data. Attendance cannot be disabled while a work session is open. Module configuration is company-specific. Modules are compiled Rust code: adding a feature requires a build, while choosing existing modules requires only an administrator action.

See [the module guide and Notes walkthrough](docs/MODULES.md). Company branding includes sage/indigo themes, a hex accent color and an optional local logo under `public/assets/`. English is the default language throughout the repository.

## Stack

| Area | Choice |
| --- | --- |
| Operational UI | Rust-generated HTML, standard forms, responsive CSS |
| Interactive demo | Leptos 0.8 SSR + WASM hydration |
| HTTP | Axum 0.8 + Tokio |
| Domain rules | Framework-independent `hph-core` crate |
| Persistence | PostgreSQL + SQLx, additive migrations and tenant RLS |
| Authentication | Argon2id password hashes, opaque hashed sessions, CSRF + Origin checks |
| Dates | UTC event storage, IANA company timezones via chrono-tz |

## Verification

```bash
cargo fmt --all --check
cargo test --workspace --features ssr --locked
cargo check --target wasm32-unknown-unknown --features hydrate --lib --locked
```

CI also runs migration/RLS tests and the real HTTP workflow against PostgreSQL using a non-owner runtime role. Browser checks cover desktop/mobile login, employee creation, clock/break flow, saved data, modules, notes, themes and logout. Node/Playwright are development-only tools for those browser checks.

## Current boundaries

This is an early portfolio application for local evaluation, not a production service certification. No public application deployment is included.

- Company bootstrap is an operator CLI action. New employee accounts can be created by an administrator; email invitations, password recovery, MFA and OIDC are not implemented.
- Existing global accounts are never overwritten by company administrators. The CLI can add another company to an existing owner account only after verifying its password.
- Corrections currently change the timestamp of an existing event; adding/removing missed events and approval workflows are future work.
- Reports count completed intervals, excluding breaks and open intervals. No overtime, leave accrual, payroll or contractual rounding is applied. Display rounds down to whole minutes; event timestamps retain seconds.
- Reads currently load company history into memory; pagination and retention controls are needed before large installations.
- The audit UI shows the latest 50 changes; records remain in the database. Refresh other tabs to see changes.
- Production operation additionally requires deployment-specific TLS, backups, monitoring, account recovery and security review.

Leave, shifts, documents, expenses and projects remain future modules. Payroll, accounting and inventory are outside this increment. The GitHub Template repository setting is unchanged; you can still clone or fork the project.

## Code map

- `crates/hph-core/`: module dependencies, permissions and time-tracking state machine.
- `src/auth.rs`: password hashing, login, session validation and CLI bootstrap.
- `src/operations.rs`: transactional company operations, permissions and date summaries.
- `src/workspace_web.rs`: authenticated HTTP routes, forms and request protections.
- `src/workspace_ui.rs`, `style/workspace.css`: operational screens and design.
- `src/workspace_tests.rs`, `scripts/browser-smoke.mjs`: database/HTTP and browser workflows.
- `src/app.rs`, `src/api.rs`, `src/server.rs`: separate Leptos demo.
- `src/database.rs`, `migrations/`: PostgreSQL infrastructure and schema.
- `harness/PROJECT_MAP.md`, `harness/PROJECT_STATE.md`: architecture and verification checkpoint.

---

<details>
<summary>🦀 Recruiter side quest: you found the Easter egg</summary>

If you made it this far, you probably followed the link on my CV—or took a very productive wrong turn on GitHub. Either way, thanks for looking beyond the bullet points.

I'm Stefano, and I'm building HPH in my spare time to learn Rust. I believe one of the best ways to learn a language is to write code and build something real—working through design decisions, testing ideas and occasionally negotiating with the Rust compiler.

Curious about the choices behind the project? [Let's talk](https://www.linkedin.com/in/stefano-contini-464933127/). Bonus points if you mention this Easter egg.

*Achievement unlocked: actually read the README.*

</details>

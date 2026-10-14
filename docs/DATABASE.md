# PostgreSQL persistence

HPH's authenticated mode uses PostgreSQL for accounts, opaque session hashes, company memberships, people, module selection, attendance, audit and notes. The explicit Leptos demo remains in-memory. See [WORKSPACE.md](WORKSPACE.md) for runnable setup and migration commands.

## Role separation

Use `HPH_MIGRATION_DATABASE_URL` only for operator CLI commands (`--migrate`, `--bootstrap`). Use `HPH_DATABASE_URL` for the server's non-owner role with NOSUPERUSER NOBYPASSRLS. Operational startup verifies those restrictions. The local Compose init script grants table and sequence access for the restricted role through the migration owner's default privileges.

Company tables use forced RLS. `Database::begin_tenant_transaction` sets `hph.user_id` and `hph.tenant_id` only for the current transaction, preventing pooled connection context leaks. RLS is a second boundary, not a replacement for membership and role checks.

Authentication tables are application-private global state: users, credentials and sessions are not exposed as direct SQL or general-purpose HTTP endpoints. The application role can operate on them for authentication. A valid session provides the user ID; company operations never trust caller-supplied identity.

## Transactions and history

`operations::begin` first locks the selected organization and verifies active membership plus active employee state. Company operations use this shared lock order so mutations, revocations, module selection and attendance checks cannot race within a company. Reads take the same lock for consistent snapshots; this intentionally favors simple correctness over high concurrency in the initial template.

Time entries retain UTC timestamps and a per-person idempotency key. State transitions use the shared core rules. Exact retries return the existing effect; reusing a key for a different event is rejected. Timestamp corrections must preserve chronological validity and cannot collide with an existing event timestamp. Audit writes share the business transaction.

Audit records are append-only through a database trigger, even for runtime roles granted UPDATE/DELETE on tables. The service records event IDs, old/new timestamps and reasons for corrections. The UI shows the latest 50 audit records. Tenant deletion and retention/export workflows are not implemented; deleting audit rows indirectly is also rejected by the trigger.

## Migrations

- `0001_company_data.sql`: organizations, global identities, people, memberships, module flags, attendance and company RLS.
- `0002_workspace.sql`: active employees, branding, password credentials, expiring sessions, immutable audit and Notes.

Migrations preserve existing data. Operational startup does not apply migrations implicitly. Run the explicit migration CLI with the owner role and retain backups appropriate to your installation.

## Existing lower-level repositories

`CompanyModuleRepository` and `MembershipRepository` remain tested building blocks. The operational service reuses the same database context and core contracts but performs membership, module and business checks together in its own transaction, avoiding nested independent transactions around writes.

## Verification

CI applies the schema and SQL isolation cases, verifies the SQLx migrator and pool, then uses a restricted runtime role for repository and full HTTP workflow tests. The operational tests cover session selection, idempotent/concurrent requests, persistence across router restart, invalid dependencies, corrections, audit immutability, role denial, deactivation, RLS, logout and expiration. Browser checks run against this database and the real Rust server using synthetic accounts.

Current reads load company event history in memory. Add pagination, retention and capacity testing before a large installation. The current checks do not constitute a deployment security assessment or backup recovery test.

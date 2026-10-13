# PostgreSQL data model

Initial schema: `migrations/0001_company_data.sql`. It creates organizations, external user identities, people, company memberships, module configuration, and attendance events.

## Tenant boundary

Organization-scoped tables enforce Row-Level Security (RLS) against `hph.tenant_id`. Each request touching company data must run in a transaction and set the verified user and selected organization with transaction-local settings:

```sql
SELECT set_config('hph.user_id', $1, true);
SELECT set_config('hph.tenant_id', $2, true);
```

The third argument must remain `true`, so both settings end with the transaction. The user ID must come from the authenticated server session; the selected organization must first be checked against an active membership. The database policies separate rows by tenant; the Rust service must also enforce active membership and role permissions before running an operation.

The production runtime role must not own these tables and must not have `BYPASSRLS`. Migrations should run with a separate owner role. Table owners normally bypass policies; `FORCE ROW LEVEL SECURITY` makes owners subject to policies, while superusers and `BYPASSRLS` roles always bypass them. CI runs the policy checks as a non-owner role without that attribute.

Composite foreign keys keep memberships and attendance events attached to a person in the same organization. An organization-scoped idempotency key prevents duplicate attendance events on request retries.

## Rust connection layer

The SSR server can open a bounded SQLx PostgreSQL pool with `HPH_DATABASE_URL`. If `HPH_MIGRATION_DATABASE_URL` is also set, startup applies migrations through that separate connection and then closes it. Use a migration owner for that URL and a runtime role without table ownership or `BYPASSRLS` for the runtime URL.

Each database operation must start a transaction scoped to a previously verified user and organization through `Database::begin_tenant_transaction(user_id, tenant_id)`. Both RLS settings are transaction-local. The `/health` endpoint checks pool reachability when configured.

## Company module repository

`src/module_repository.rs` provides tenant-scoped reads and atomic writes for `hph_company_modules`. Reads require an active membership; writes additionally require an active admin membership. The repository validates module dependencies through `hph-core` and records all flags together in one transaction. It returns `None` when no company selection is stored so the caller can apply its configured preset.

Its PostgreSQL CI integration test seeds fixtures through a separate admin connection, then exercises repository reads and writes through a non-owner runtime role without `BYPASSRLS`.

The SSR screens and server functions do not call this repository yet. The current application still uses its in-memory demo actor, whose IDs are not database UUIDs. Connect it only after authentication supplies verified user and organization UUIDs; do not adapt the synthetic demo identity into a production database identity.

## Current limit

People, attendance, memberships, and module configuration in the screens are still in-memory demo state. Authentication, session verification, HTTP integration, and production authorization remain out of scope. SQLx repository behavior is exercised against PostgreSQL in CI.

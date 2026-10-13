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

## Current limit

This is a tested schema foundation. The Rust app does not yet connect to PostgreSQL: SQLx pool management, migrations at startup, transactional tenant context, and repositories are the next implementation increment. The schema does not provide authentication, admin-only permissions, payroll rules, or production readiness by itself.

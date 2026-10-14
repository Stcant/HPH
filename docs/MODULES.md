# Extending HPH

## Shared contract

`crates/hph-core/src/lib.rs` owns `ModuleId`, `catalog()` and `ModuleSet`. Dependencies are checked on the server. People has none, Attendance requires People, Reports requires Attendance, and Notes requires People. An invalid configuration leaves the previous selection intact.

Modules are compiled Rust features of the application, not untrusted runtime plugins. Each authenticated company persists its own selection. HPH Core includes sessions, membership checks, company settings and audit. Disabling a module hides and blocks its operations without deleting data; open sessions prevent Attendance from being disabled.

The explicit Leptos demo still uses `hph.toml` and in-memory settings. Notes belongs to the authenticated workspace and is not offered in the demo.

## A complete example: Team notes

| Layer | Existing implementation | Purpose |
| --- | --- | --- |
| Catalog | `ModuleId::Notes`, `catalog()` | Typed identity, description and People dependency |
| Schema | `migrations/0002_workspace.sql`, `hph_notes` | Company/author ownership, length constraint and tenant RLS |
| Service | `operations::add_note` | Active membership, enabled-module check, validation and audit in one transaction |
| Read | `operations::snapshot` | Company-scoped latest 50 notes, only if enabled |
| HTTP | `/notes/add` in `workspace_web.rs` | Same-origin POST, session/company CSRF, authenticated actor |
| Screen | `workspace_ui::notes` | Escaped user text, accessible form, empty collection and saved-state feedback |
| Selection | `operations::set_modules` | Persisted flags and atomic dependency validation |
| Checks | `workspace_tests.rs`, `browser-smoke.mjs` | Real database, tenant checks, enabled workflow and HTML escaping |

Follow the flow from the route to service before copying code. Obtain identity from the session, never from a hidden user ID. `operations::begin` locks the company and rechecks membership/active employee state inside the transaction. Every mutation, including role changes and deactivation, follows the same locking order. Reuse that transaction for the module write and audit record.

Notes deliberately supports append-only messages without editing/deletion or notifications. It demonstrates the complete extension path without adding those unrelated workflows.

## Add your module

1. Add a `ModuleId` and catalog definition with explicit, acyclic dependencies. Update mappings and company module persistence. If the SQL check constraint contains the module IDs, extend it through a **new** migration.
2. Define domain contracts in `hph-core` when they are independent of HTTP/database/UI. Document behavior and what must happen if the module is disabled during an open operation.
3. Add tenant-owned tables with constraints and RLS. Never alter an applied migration, seed real installations with demo records, or erase data on module disable.
4. Add a transaction-scoped service operation. Recheck active membership, role, module activation and input invariants before writes. Use server identity and timestamps where the workflow requires them.
5. Add the protected route and Rust-rendered view. Reuse existing design tokens, navigation and form helpers. Pass every dynamic value through HTML escaping. Keep prose, accessibility labels, errors, comments and documentation in English.
6. Test success, a foreign tenant, an employee without the required role, disabled-module calls, invalid input and retained data. Exercise PostgreSQL with a non-owner runtime role and include a browser path when new interaction is introduced.
7. Document the module's setup and actual limits. Update presets only if the module should really be included by default.

Modules and permissions solve different problems: a company may enable a feature while only some members have permission to mutate it. A hidden menu or disabled button is not authorization.

## Future leave module

People is the likely dependency. Specify date ranges, company timezone and working calendar, approvers, overlaps, corrections and balances before implementation. Do not infer contract, payroll or legal rules. This module is not implemented by the Notes example.

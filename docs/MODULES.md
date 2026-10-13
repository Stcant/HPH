# Extending HPH

## Current contract

Module composition is declared in `crates/hph-core/src/lib.rs`: `ModuleId`, `catalog()` and `ModuleSet`. This shared source is compiled for both the server and the browser. Do not maintain a second JavaScript catalog or a separate dependency engine in the UI.

Module selection is a company setting: HPH Core is always included, and each company enables the features that fit its processes. Modules remain compiled, versioned Rust code within the template; they are not third-party plugins loaded at runtime. The demo keeps the selection in memory; persistent application configuration must be associated with the authenticated organization.

Core is always present. The current modules form a dependency chain: People → Attendance → Reports. A module's dependencies must be enabled before enabling it. Before disabling a dependency, disable the modules that use it. An invalid change must leave the previous state intact.

Disabling a module must stop its protected operations and hide the relevant screens **without deleting its data**. Open work sessions prevent Attendance from being disabled.

## Customize a single company installation

In `hph.toml`, set `modules` to an explicit list of IDs (`people`, `attendance`, `reports`). This optional list overrides `preset` and is validated at startup: missing dependencies or duplicate IDs cause the configuration to be rejected. A commented example is included in `hph.toml`.

This is persistent initial configuration for the company installation. Changes made from the demo screen remain temporary and reset on restart. Persisting runtime settings per organization requires connecting the PostgreSQL repository to authenticated requests.

## Add a module

1. Add a typed identifier to `ModuleId` and a definition to `catalog()` with explicit dependencies. Do not introduce cycles.
2. Define domain rules and serialized types in the core or, as it grows, a dedicated crate. Keep Leptos, HTTP and database concerns out of pure domain rules.
3. Implement the server operation in `src/api.rs`. Obtain the actor from server context and verify membership, permissions and module activation before mutating state. A hidden button is not an access control.
4. Add the screen and navigation in `src/app.rs`. Reuse existing colors, components, empty states and errors. Keep interface text, accessibility labels, comments and documentation in English.
5. Define what happens to open operations when the module is disabled. Do not implicitly delete documents or events.
6. Test domain rules and the HTTP boundary: valid operations, denied permissions, foreign companies, disabled modules, invalid input and data retention.
7. Update presets and documentation only if the new module actually belongs to that preset. Do not present an unimplemented module as available.

## Future example: leave

Dependency: People. Clarify these requirements before implementation: date ranges, company time zone, working calendar, approver, overlaps and corrections. A request belongs to a company and employee; approval requires a permission distinct from submission. Leave balances require explicit rules and must not be inferred from assumptions about employment contracts.

## Demo boundary

`DemoState` uses mutex-protected shared memory and a fixed actor. Domain-level tenant checks do not establish a secure multi-company service. Authentication, per-request membership verification and database isolation at the HTTP boundary still need to be integrated.

PostgreSQL operations must use transactions for related changes, constraints to prevent incompatible work sessions, tenant-linked keys and isolation tests with at least two companies. Module activation is a company setting, while permissions belong to the user: both must be checked.

Use versioned migrations to preserve configuration and data as the template evolves. Do not overwrite an existing installation with demo data.

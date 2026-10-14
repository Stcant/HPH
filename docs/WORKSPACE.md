# Run an authenticated company workspace

## Local PostgreSQL

The root `compose.yaml` starts PostgreSQL 18 on **127.0.0.1:5432** and preserves data in a named Docker volume. `scripts/init-runtime.sh` creates a restricted `hph_runtime` role and default grants for tables and sequences created by `hph_owner`.

```bash
export HPH_POSTGRES_PASSWORD='replace-with-a-local-owner-password'
export HPH_RUNTIME_PASSWORD='replace-with-a-different-runtime-password'
docker compose up -d
```

Set connection URLs. Percent-encode reserved URL characters in passwords; the sample passwords above need no encoding.

```bash
export HPH_MIGRATION_DATABASE_URL="postgres://hph_owner:${HPH_POSTGRES_PASSWORD}@127.0.0.1:5432/hph"
export HPH_DATABASE_URL="postgres://hph_runtime:${HPH_RUNTIME_PASSWORD}@127.0.0.1:5432/hph"
export HPH_ORIGIN='http://127.0.0.1:3000'
```

The two roles are intentionally separate. Normal web startup rejects a superuser, BYPASSRLS role or owner of an HPH table. The migration connection is used only by explicit CLI commands in operational mode.

The init script runs only when PostgreSQL initializes a new volume. Changing environment variables later does not rotate existing database passwords. Preserve your volume; do not delete it to upgrade HPH.

## First company and account

```bash
export HPH_BOOTSTRAP_EMAIL='you@example.com'
export HPH_BOOTSTRAP_NAME='Your name'
export HPH_BOOTSTRAP_COMPANY='Your company'
export HPH_BOOTSTRAP_SLUG='your-company'
read -rsp 'Administrator password (12+ characters): ' HPH_BOOTSTRAP_PASSWORD; echo
export HPH_BOOTSTRAP_PASSWORD
cargo run --features ssr --locked -- --bootstrap
unset HPH_BOOTSTRAP_PASSWORD
cargo run --features ssr --locked
```

The bootstrap command applies pending migrations and creates the company, administrator profile and People/Attendance/Reports selection. It does not start the web server. Reusing a company slug is rejected, not treated as permission to overwrite data.

Open **http://127.0.0.1:3000**, sign in and choose your company. Use precisely the hostname configured in `HPH_ORIGIN`; switching between localhost and 127.0.0.1 will cause mutation origin checks to reject the request.

To add another company for the same administrator, run bootstrap with a new company name/slug and the existing account email/password. The password is verified; bootstrap never silently resets it.

For PowerShell, use `$env:NAME = 'value'` instead of `export`. Supply passwords through your shell's secure-input workflow and clear the temporary bootstrap variable afterwards.

## Everyday workflow

1. Open **People**. Add a name, unique employee number, department and job title. Optionally set an email and initial password to create a login. Share credentials through your own secure channel; HPH sends no messages.
2. Employees sign in with that login, choose their company and clock in from Overview or Attendance. Employees can see only their own profile and time entries; administrators can review the company.
3. Start/end breaks and clock out. An open interval is not counted until closed. Retrying an identical request key creates one event only; invalid transitions are rejected.
4. Use **Reports** to select a month and optionally a day. Midnight and daylight saving boundaries follow the company's IANA timezone.
5. Administrators can expand **Correct time** on an existing event. Enter an ISO 8601 timestamp with offset, such as `2026-10-14T09:00:00+02:00`, and a reason. The corrected sequence must remain valid; future times and timestamp collisions are rejected. The event ID and original/replacement timestamps are stored in audit history.
6. Use **Workspace** for branding, modules and audit. Use **My account** to change your password; this revokes all your sessions.

An administrator cannot deactivate or demote their own account. Open work sessions must be closed before deactivating a person or disabling Attendance. Deactivation preserves history and is enforced on the next company operation. Changing a profile's contact email does not change the login email. A company administrator cannot overwrite another existing account's credentials.

## Appearance and modules

Choose sage or indigo, a six-digit hex accent color and a local logo path. Put installation-owned images under `public/assets/`; for example `/assets/hph-mark.svg`. Remote image URLs, data URLs and path traversal are rejected. Logos are deployed files, not uploads.

Team notes is a working optional module. Enable People + Team notes, open the note screen, and publish a company note. Disabling Notes keeps existing notes. The [module walkthrough](MODULES.md) traces its catalog, schema, service, route and screen.

## Upgrades and request security

```bash
cargo run --features ssr --locked -- --migrate
cargo run --features ssr --locked
```

Back up your database before schema upgrades. Versioned migrations are additive; never edit an applied migration. `--migrate` uses `HPH_MIGRATION_DATABASE_URL`, while web requests use only `HPH_DATABASE_URL`.

Session cookies are HttpOnly, SameSite=Strict, and Secure for HTTPS origins. Tokens have 256 bits of entropy; only their SHA-256 hashes are stored. Sessions expire after eight hours. POST requests require an exact Origin match, and authenticated forms additionally carry a session/company-bound CSRF value. Switching companies invalidates forms opened for the previous company.

Logins use Argon2id, generic credential errors, per-account lockouts, a bounded per-IP attempt window and a concurrency limit. Client IP comes from the direct connection; forwarded headers are not trusted. A reverse proxy therefore needs its own suitable login rate limit.

HTTP development binding is loopback-only. For a future TLS proxy deployment, set an HTTPS `HPH_ORIGIN` and appropriate `HPH_BIND`; HPH does not terminate TLS itself. This setup has not been deployed publicly or certified for production use.

## Test boundaries

Ordinary `cargo test` runs unit/HTTP demo tests; PostgreSQL integration tests return early unless `HPH_TEST_DATABASE_URL` and `HPH_TEST_ADMIN_DATABASE_URL` are set. CI supplies an isolated database and restricted runtime role, then exercises the operational HTTP workflow. Its browser captures use synthetic accounts only. Browser dependencies are not runtime dependencies.

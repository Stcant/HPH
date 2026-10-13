-- PostgreSQL schema foundation for company-scoped HPH data.
-- Runtime requests must set hph.tenant_id and hph.user_id transaction-locally.
CREATE FUNCTION hph_current_tenant_id() RETURNS uuid
LANGUAGE sql STABLE PARALLEL SAFE
AS $$ SELECT NULLIF(current_setting('hph.tenant_id', true), '')::uuid $$;

CREATE FUNCTION hph_current_user_id() RETURNS uuid
LANGUAGE sql STABLE PARALLEL SAFE
AS $$ SELECT NULLIF(current_setting('hph.user_id', true), '')::uuid $$;

CREATE TABLE hph_organizations (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    slug text NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
    name text NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 160),
    timezone text NOT NULL DEFAULT 'Europe/Rome',
    theme text NOT NULL DEFAULT 'sage' CHECK (theme IN ('sage', 'indigo')),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE hph_users (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email text,
    display_name text NOT NULL CHECK (length(trim(display_name)) BETWEEN 1 AND 160),
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX hph_users_email_lower_unique
    ON hph_users (lower(email)) WHERE email IS NOT NULL;

-- Provider/subject stays opaque so an OIDC provider can be selected per deployment.
CREATE TABLE hph_user_identities (
    provider text NOT NULL CHECK (length(trim(provider)) BETWEEN 1 AND 80),
    subject text NOT NULL CHECK (length(trim(subject)) BETWEEN 1 AND 255),
    user_id uuid NOT NULL REFERENCES hph_users(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (provider, subject)
);
CREATE INDEX hph_user_identities_user_idx ON hph_user_identities (user_id);

CREATE TABLE hph_people (
    organization_id uuid NOT NULL REFERENCES hph_organizations(id) ON DELETE CASCADE,
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    employee_number text NOT NULL CHECK (length(trim(employee_number)) BETWEEN 1 AND 80),
    display_name text NOT NULL CHECK (length(trim(display_name)) BETWEEN 1 AND 160),
    email text,
    department text,
    job_title text,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, id),
    UNIQUE (organization_id, employee_number)
);
CREATE INDEX hph_people_email_lower_idx
    ON hph_people (organization_id, lower(email)) WHERE email IS NOT NULL;

CREATE TABLE hph_memberships (
    organization_id uuid NOT NULL REFERENCES hph_organizations(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES hph_users(id) ON DELETE CASCADE,
    person_id uuid NOT NULL,
    role text NOT NULL CHECK (role IN ('employee', 'admin')),
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, user_id),
    UNIQUE (organization_id, person_id),
    FOREIGN KEY (organization_id, person_id)
        REFERENCES hph_people (organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX hph_memberships_user_active_idx
    ON hph_memberships (user_id, active, organization_id);

CREATE TABLE hph_company_modules (
    organization_id uuid NOT NULL REFERENCES hph_organizations(id) ON DELETE CASCADE,
    module_id text NOT NULL CHECK (module_id IN ('people', 'attendance', 'reports')),
    enabled boolean NOT NULL DEFAULT true,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, module_id)
);

CREATE TABLE hph_attendance_events (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id uuid NOT NULL REFERENCES hph_organizations(id) ON DELETE CASCADE,
    person_id uuid NOT NULL,
    created_by_user_id uuid NOT NULL,
    kind text NOT NULL CHECK (kind IN ('in', 'break_start', 'break_end', 'out')),
    occurred_at timestamptz NOT NULL,
    idempotency_key uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (organization_id, person_id)
        REFERENCES hph_people (organization_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (organization_id, created_by_user_id)
        REFERENCES hph_memberships (organization_id, user_id) ON DELETE RESTRICT,
    UNIQUE (organization_id, person_id, idempotency_key)
);
CREATE INDEX hph_attendance_person_time_idx
    ON hph_attendance_events (organization_id, person_id, occurred_at DESC);

ALTER TABLE hph_organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE hph_organizations FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_scope ON hph_organizations
    USING (id = hph_current_tenant_id())
    WITH CHECK (id = hph_current_tenant_id());

ALTER TABLE hph_people ENABLE ROW LEVEL SECURITY;
ALTER TABLE hph_people FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_scope ON hph_people
    USING (organization_id = hph_current_tenant_id())
    WITH CHECK (organization_id = hph_current_tenant_id());

ALTER TABLE hph_company_modules ENABLE ROW LEVEL SECURITY;
ALTER TABLE hph_company_modules FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_scope ON hph_company_modules
    USING (organization_id = hph_current_tenant_id())
    WITH CHECK (organization_id = hph_current_tenant_id());

ALTER TABLE hph_attendance_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE hph_attendance_events FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_scope ON hph_attendance_events
    USING (organization_id = hph_current_tenant_id())
    WITH CHECK (organization_id = hph_current_tenant_id());

ALTER TABLE hph_memberships ENABLE ROW LEVEL SECURITY;
ALTER TABLE hph_memberships FORCE ROW LEVEL SECURITY;
-- A user may list their own company memberships; all writes stay tenant-scoped.
CREATE POLICY membership_read ON hph_memberships
    FOR SELECT USING (
        organization_id = hph_current_tenant_id()
        OR user_id = hph_current_user_id()
    );
CREATE POLICY membership_write ON hph_memberships
    FOR ALL
    USING (organization_id = hph_current_tenant_id())
    WITH CHECK (organization_id = hph_current_tenant_id());

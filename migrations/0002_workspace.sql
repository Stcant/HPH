-- Additive migration: existing demo/schema data is preserved.
ALTER TABLE hph_people ADD COLUMN active boolean NOT NULL DEFAULT true;
ALTER TABLE hph_organizations ADD COLUMN logo_path text NOT NULL DEFAULT '';
ALTER TABLE hph_organizations ADD COLUMN accent text NOT NULL DEFAULT '#173d32'
    CHECK (accent ~ '^#[0-9a-fA-F]{6}$');

CREATE TABLE hph_credentials (
    user_id uuid PRIMARY KEY REFERENCES hph_users(id) ON DELETE CASCADE,
    password_hash text NOT NULL,
    failed_attempts integer NOT NULL DEFAULT 0,
    locked_until timestamptz,
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE hph_sessions (
    token_hash text PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES hph_users(id) ON DELETE CASCADE,
    organization_id uuid REFERENCES hph_organizations(id) ON DELETE CASCADE,
    expires_at timestamptz NOT NULL DEFAULT now() + interval '8 hours'
);
CREATE INDEX hph_sessions_expiry_idx ON hph_sessions(expires_at);
CREATE TABLE hph_audit (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES hph_organizations(id) ON DELETE CASCADE,
    actor_user_id uuid NOT NULL REFERENCES hph_users(id),
    action text NOT NULL,
    detail text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE hph_audit ENABLE ROW LEVEL SECURITY;
ALTER TABLE hph_audit FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_scope ON hph_audit
    USING (organization_id = hph_current_tenant_id())
    WITH CHECK (organization_id = hph_current_tenant_id());
-- Audit rows are append-only, including for runtime roles with broad table grants.
CREATE FUNCTION hph_audit_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'Audit records are immutable'; END $$;
CREATE TRIGGER hph_audit_immutable BEFORE UPDATE OR DELETE ON hph_audit
    FOR EACH ROW EXECUTE FUNCTION hph_audit_immutable();

ALTER TABLE hph_company_modules DROP CONSTRAINT hph_company_modules_module_id_check;
ALTER TABLE hph_company_modules ADD CONSTRAINT hph_company_modules_module_id_check
    CHECK (module_id IN ('people', 'attendance', 'reports', 'notes'));
CREATE TABLE hph_notes (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id uuid NOT NULL REFERENCES hph_organizations(id) ON DELETE CASCADE,
    author_user_id uuid NOT NULL REFERENCES hph_users(id),
    body text NOT NULL CHECK (length(trim(body)) BETWEEN 1 AND 2000),
    created_at timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE hph_notes ENABLE ROW LEVEL SECURITY;
ALTER TABLE hph_notes FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_scope ON hph_notes
    USING (organization_id = hph_current_tenant_id())
    WITH CHECK (organization_id = hph_current_tenant_id());

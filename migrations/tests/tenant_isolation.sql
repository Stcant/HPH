\set ON_ERROR_STOP on

CREATE ROLE hph_test_runtime NOLOGIN NOSUPERUSER NOBYPASSRLS;
GRANT USAGE ON SCHEMA public TO hph_test_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON
    hph_organizations, hph_people, hph_memberships,
    hph_company_modules, hph_attendance_events
    TO hph_test_runtime;

INSERT INTO hph_organizations (id, slug, name) VALUES
('10000000-0000-4000-8000-000000000001', 'company-a', 'Company A'),
('10000000-0000-4000-8000-000000000002', 'company-b', 'Company B');

INSERT INTO hph_users (id, email, display_name) VALUES
('20000000-0000-4000-8000-000000000001', 'a@example.test', 'User A'),
('20000000-0000-4000-8000-000000000002', 'b@example.test', 'User B');

INSERT INTO hph_people (organization_id, id, employee_number, display_name) VALUES
('10000000-0000-4000-8000-000000000001', '30000000-0000-4000-8000-000000000001', 'A-1', 'Person A'),
('10000000-0000-4000-8000-000000000002', '30000000-0000-4000-8000-000000000002', 'B-1', 'Person B');

INSERT INTO hph_memberships (organization_id, user_id, person_id, role) VALUES
('10000000-0000-4000-8000-000000000001', '20000000-0000-4000-8000-000000000001', '30000000-0000-4000-8000-000000000001', 'admin'),
('10000000-0000-4000-8000-000000000002', '20000000-0000-4000-8000-000000000001', '30000000-0000-4000-8000-000000000002', 'employee'),
('10000000-0000-4000-8000-000000000002', '20000000-0000-4000-8000-000000000002', '30000000-0000-4000-8000-000000000002', 'admin');

INSERT INTO hph_company_modules (organization_id, module_id) VALUES
('10000000-0000-4000-8000-000000000001', 'people'),
('10000000-0000-4000-8000-000000000002', 'people');

INSERT INTO hph_attendance_events
    (organization_id, person_id, created_by_user_id, kind, occurred_at, idempotency_key)
VALUES
('10000000-0000-4000-8000-000000000001', '30000000-0000-4000-8000-000000000001',
 '20000000-0000-4000-8000-000000000001', 'in', '2026-01-01T09:00:00Z',
 '40000000-0000-4000-8000-000000000001'),
('10000000-0000-4000-8000-000000000002', '30000000-0000-4000-8000-000000000002',
 '20000000-0000-4000-8000-000000000001', 'in', '2026-01-01T09:00:00Z',
 '40000000-0000-4000-8000-000000000002');

SET ROLE hph_test_runtime;
BEGIN;
SELECT set_config('hph.user_id', '20000000-0000-4000-8000-000000000001', true);
SELECT set_config('hph.tenant_id', '10000000-0000-4000-8000-000000000001', true);

DO $$
DECLARE row_count integer;
BEGIN
    SELECT count(*) INTO row_count FROM hph_organizations;
    IF row_count <> 1 THEN RAISE EXCEPTION 'organization RLS exposed % rows', row_count; END IF;

    SELECT count(*) INTO row_count FROM hph_people;
    IF row_count <> 1 THEN RAISE EXCEPTION 'people RLS exposed % rows', row_count; END IF;

    SELECT count(*) INTO row_count FROM hph_memberships;
    IF row_count <> 2 THEN RAISE EXCEPTION 'membership visibility returned % rows, expected own memberships', row_count; END IF;

    SELECT count(*) INTO row_count FROM hph_company_modules;
    IF row_count <> 1 THEN RAISE EXCEPTION 'module RLS exposed % rows', row_count; END IF;

    SELECT count(*) INTO row_count FROM hph_attendance_events;
    IF row_count <> 1 THEN RAISE EXCEPTION 'attendance RLS exposed % rows', row_count; END IF;

    UPDATE hph_memberships
       SET role = 'admin'
     WHERE organization_id = '10000000-0000-4000-8000-000000000002'
       AND user_id = '20000000-0000-4000-8000-000000000001';
    GET DIAGNOSTICS row_count = ROW_COUNT;
    IF row_count <> 0 THEN RAISE EXCEPTION 'cross-company membership update unexpectedly succeeded'; END IF;

    DELETE FROM hph_memberships
     WHERE organization_id = '10000000-0000-4000-8000-000000000002'
       AND user_id = '20000000-0000-4000-8000-000000000001';
    GET DIAGNOSTICS row_count = ROW_COUNT;
    IF row_count <> 0 THEN RAISE EXCEPTION 'cross-company membership delete unexpectedly succeeded'; END IF;

    BEGIN
        INSERT INTO hph_people (organization_id, employee_number, display_name)
        VALUES ('10000000-0000-4000-8000-000000000002', 'A-LEAK', 'Should be blocked');
        RAISE EXCEPTION 'cross-company insert unexpectedly succeeded';
    EXCEPTION WHEN insufficient_privilege THEN
        NULL;
    END;
END $$;
COMMIT;
RESET ROLE;

SET ROLE hph_test_runtime;
BEGIN;
SELECT set_config('hph.user_id', '20000000-0000-4000-8000-000000000001', true);
DO $$
DECLARE row_count integer;
BEGIN
    SELECT count(*) INTO row_count FROM hph_people;
    IF row_count <> 0 THEN RAISE EXCEPTION 'rows visible without tenant context'; END IF;
END $$;
COMMIT;
RESET ROLE;

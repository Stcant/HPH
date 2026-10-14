//! Integration tests run against a disposable PostgreSQL database and restricted runtime role.
use super::*;
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn request(
    app: Router,
    path: &str,
    body: Option<&[(&str, &str)]>,
    cookie: Option<&str>,
    origin: bool,
) -> (StatusCode, HeaderMap, String) {
    let mut request = Request::builder().uri(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    if origin {
        request = request.header(header::ORIGIN, "http://127.0.0.1:3000");
    }
    let encoded = body
        .map(|f| serde_urlencoded::to_string(f).unwrap())
        .unwrap_or_default();
    if body.is_some() {
        request = request
            .method("POST")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
    }
    let response = app
        .oneshot(request.body(Body::from(encoded)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, String::from_utf8(bytes.to_vec()).unwrap())
}
async fn sign_in(app: Router, email: &str) -> String {
    let (status, headers, body) = request(
        app,
        "/auth/login",
        Some(&[
            ("email", email),
            ("password", "correct horse battery staple"),
        ]),
        None,
        true,
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER, "{body}");
    let cookie = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    assert!(cookie.contains("HttpOnly; SameSite=Strict"));
    cookie.split(';').next().unwrap().to_owned()
}
#[tokio::test]
async fn operational_http_workflow_is_persistent_scoped_and_authorized() {
    let Ok(runtime_url) = std::env::var("HPH_TEST_DATABASE_URL") else {
        return;
    };
    let admin = Database::connect(&std::env::var("HPH_TEST_ADMIN_DATABASE_URL").unwrap())
        .await
        .unwrap();
    admin.migrate().await.unwrap();
    let suffix = auth::random_token().unwrap();
    let suffix = &suffix[..12];
    let email = format!("owner-{suffix}@example.test");
    let company = format!("Workspace {suffix}");
    auth::bootstrap(
        &admin,
        email.clone(),
        "correct horse battery staple".into(),
        "Test Owner".into(),
        company.clone(),
        format!("workspace-{suffix}"),
    )
    .await
    .unwrap();
    let runtime = Database::connect(&runtime_url).await.unwrap();
    runtime.verify_runtime_role().await.unwrap();
    assert!(admin.verify_runtime_role().await.is_err());
    let state = WebState::new(runtime.clone(), "http://127.0.0.1:3000".into())
        .await
        .unwrap();
    let app = router(state.clone());
    assert_eq!(
        request(app.clone(), "/app", None, None, false).await.0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        request(
            app.clone(),
            "/auth/login",
            Some(&[("email", &email), ("password", "wrong")]),
            None,
            false
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let cookie = sign_in(app.clone(), &email).await;
    let token = cookie.strip_prefix("hph_session=").unwrap();
    let mut session = auth::session(&runtime, token).await.unwrap();
    let companies = op::companies(&runtime, &session).await.unwrap();
    assert_eq!(companies.len(), 1);
    let tenant = companies[0].0;
    let fields = [
        ("company", tenant.to_string()),
        ("csrf", session.csrf.clone()),
    ];
    assert_eq!(
        request(
            app.clone(),
            "/company/select",
            Some(&[("company", &fields[0].1)]),
            Some(&cookie),
            true
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, _, body) = request(
        app.clone(),
        "/company/select",
        Some(&[("company", &fields[0].1), ("csrf", &fields[1].1)]),
        Some(&cookie),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER, "{body}");
    session = auth::session(&runtime, token).await.unwrap();
    let (_, _, html) = request(app.clone(), "/app", None, Some(&cookie), false).await;
    assert!(html.contains(&company));
    assert!(html.contains("Test Owner"));
    let key = op::new_id().unwrap();
    let (a, b) = tokio::join!(
        op::punch(&runtime, &session, hph_core::PunchKind::In, key),
        op::punch(&runtime, &session, hph_core::PunchKind::In, key)
    );
    a.unwrap();
    b.unwrap();
    let data = op::snapshot(&runtime, &session).await.unwrap();
    assert_eq!(data.events.len(), 1);
    assert!(
        op::punch(&runtime, &session, hph_core::PunchKind::Out, key)
            .await
            .is_err()
    );
    assert!(
        op::punch(
            &runtime,
            &session,
            hph_core::PunchKind::In,
            op::new_id().unwrap()
        )
        .await
        .is_err()
    );
    let event = data.events[0].id;
    let at = chrono::Utc::now().timestamp() - 3600;
    op::correct(
        &runtime,
        &session,
        event,
        at,
        "Corrected forgotten start time",
    )
    .await
    .unwrap();
    assert!(
        op::correct(&runtime, &session, event, at, "")
            .await
            .is_err()
    );
    op::punch(
        &runtime,
        &session,
        hph_core::PunchKind::Out,
        op::new_id().unwrap(),
    )
    .await
    .unwrap();
    assert!(
        op::correct(
            &runtime,
            &session,
            event,
            chrono::Utc::now().timestamp() + 100,
            "Invalid future"
        )
        .await
        .is_err()
    );
    let data = op::snapshot(&runtime, &session).await.unwrap();
    assert_eq!(data.events.len(), 2);
    assert!(
        data.audit
            .iter()
            .any(|r| r.0 == "attendance.corrected" && r.1.contains("Corrected forgotten"))
    );
    // A new process/router reads the same data and existing session from PostgreSQL.
    let app2 = router(
        WebState::new(
            Database::connect(&runtime_url).await.unwrap(),
            "http://127.0.0.1:3000".into(),
        )
        .await
        .unwrap(),
    );
    let (status, _, html) = request(app2, "/app?tab=attendance", None, Some(&cookie), false).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Clock out"));
    assert!(
        op::set_modules(&runtime, &session, vec!["reports".into()])
            .await
            .is_err()
    );
    op::set_modules(&runtime, &session, vec!["people".into(), "notes".into()])
        .await
        .unwrap();
    assert!(
        op::punch(
            &runtime,
            &session,
            hph_core::PunchKind::In,
            op::new_id().unwrap()
        )
        .await
        .is_err()
    );
    op::add_note(&runtime, &session, "<script>alert('x')</script>")
        .await
        .unwrap();
    let (_, _, html) = request(app.clone(), "/app?tab=notes", None, Some(&cookie), false).await;
    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script>"));
    let employee_email = format!("employee-{suffix}@example.test");
    op::save_person(
        &runtime,
        &session,
        op::PersonInput {
            id: None,
            number: "E001".into(),
            name: "Test Employee".into(),
            email: employee_email.clone(),
            department: "Design".into(),
            job: "Designer".into(),
            active: true,
            role: "employee".into(),
            password: "correct horse battery staple".into(),
        },
    )
    .await
    .unwrap();
    let employee_cookie = sign_in(app.clone(), &employee_email).await;
    let mut employee = auth::session(
        &runtime,
        employee_cookie.strip_prefix("hph_session=").unwrap(),
    )
    .await
    .unwrap();
    op::select_company(&runtime, &employee, tenant)
        .await
        .unwrap();
    employee.tenant = Some(tenant);
    let employee_data = op::snapshot(&runtime, &employee).await.unwrap();
    assert_eq!(employee_data.people.len(), 1);
    assert!(op::set_modules(&runtime, &employee, vec![]).await.is_err());
    assert!(
        op::correct(&runtime, &employee, event, at, "Unauthorized")
            .await
            .is_err()
    );
    assert_eq!(
        request(
            app.clone(),
            "/app?tab=settings",
            None,
            Some(&employee_cookie),
            false
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // Unknown/foreign company cannot be selected even with a valid authenticated session.
    let foreign = op::new_id().unwrap();
    assert!(
        op::select_company(&runtime, &employee, foreign)
            .await
            .is_err()
    );
    let mut foreign_session = employee.clone();
    foreign_session.tenant = Some(foreign);
    assert!(op::snapshot(&runtime, &foreign_session).await.is_err());
    let person = &employee_data.people[0];
    op::save_person(
        &runtime,
        &session,
        op::PersonInput {
            id: Some(person.id),
            number: person.number.clone(),
            name: person.name.clone(),
            email: person.email.clone(),
            department: person.department.clone(),
            job: person.job.clone(),
            active: false,
            role: "employee".into(),
            password: String::new(),
        },
    )
    .await
    .unwrap();
    assert!(op::snapshot(&runtime, &employee).await.is_err());
    // RLS denies a tenant-less read, and audit rows cannot be edited by the runtime role.
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM hph_people")
        .fetch_one(runtime.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
    let mut tx = runtime
        .begin_tenant_transaction(session.user, tenant)
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE hph_audit SET detail='tampered' WHERE organization_id=$1")
            .bind(tenant)
            .execute(&mut *tx)
            .await
            .is_err()
    );
    drop(tx);
    let (status, _, _) = request(
        app.clone(),
        "/auth/logout",
        Some(&[("csrf", &session.csrf)]),
        Some(&cookie),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert!(auth::session(&runtime, token).await.is_err());
    // Expiration is enforced on every request.
    let cookie = sign_in(app.clone(), &email).await;
    let token = cookie.strip_prefix("hph_session=").unwrap();
    sqlx::query("UPDATE hph_sessions SET expires_at=now()-interval '1 second' WHERE token_hash=$1")
        .bind(auth::digest(token))
        .execute(admin.pool())
        .await
        .unwrap();
    assert!(auth::session(&runtime, token).await.is_err());
    runtime.close().await;
    admin.close().await;
}

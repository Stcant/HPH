//! Authenticated HTTP boundary. All mutations are same-origin POSTs with CSRF validation.
use crate::{
    auth::{self, Session},
    database::Database,
    operations::{self as op, AppError, AppResult},
    workspace_ui as ui,
};
use axum::{
    Extension, Form, Router,
    body::Body,
    extract::{ConnectInfo, DefaultBodyLimit, Query, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub struct WebState {
    pub db: Database,
    pub origin: String,
    pub secure: bool,
    dummy: String,
    login_slots: tokio::sync::Semaphore,
    attempts: Mutex<HashMap<std::net::IpAddr, (Instant, u32)>>,
}
impl WebState {
    pub async fn new(db: Database, origin: String) -> AppResult<Arc<Self>> {
        let uri = origin
            .parse::<axum::http::Uri>()
            .map_err(|_| AppError::Invalid("Invalid HPH_ORIGIN."))?;
        let secure = uri.scheme_str() == Some("https");
        let local = matches!(uri.host(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if (!secure && !(uri.scheme_str() == Some("http") && local))
            || uri.authority().is_none()
            || uri.path() != "/"
            || uri.query().is_some()
            || origin.ends_with('/')
        {
            return Err(AppError::Invalid(
                "HPH_ORIGIN must be an HTTPS origin, or HTTP on localhost, without a trailing slash.",
            ));
        }
        Ok(Arc::new(Self {
            db,
            origin,
            secure,
            dummy: auth::password_hash(auth::random_token()?).await?,
            login_slots: tokio::sync::Semaphore::new(4),
            attempts: Mutex::new(HashMap::new()),
        }))
    }
}
pub fn router(state: Arc<WebState>) -> Router {
    Router::new()
        .route("/", get(|| async { Redirect::to("/app") }))
        .route("/health", get(|| async { "HPH workspace ready" }))
        .route("/login", get(|| async { Html(ui::login()) }))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/password", post(password))
        .route("/companies", get(companies))
        .route("/company/select", post(select_company))
        .route("/app", get(workspace))
        .route("/people/save", post(save_person))
        .route("/attendance/punch", post(punch))
        .route("/attendance/correct", post(correct))
        .route("/settings/save", post(settings))
        .route("/modules/save", post(modules))
        .route("/notes/add", post(note))
        .route(
            "/workspace.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css")],
                    include_str!("../style/workspace.css"),
                )
            }),
        )
        .nest_service(
            "/assets",
            tower_http::services::ServeDir::new("public/assets"),
        )
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), security))
        .with_state(state)
}
async fn security(
    State(s): State<Arc<WebState>>,
    request: axum::http::Request<Body>,
    next: Next,
) -> Response {
    if request.method() == axum::http::Method::POST
        && request
            .headers()
            .get(header::ORIGIN)
            .and_then(|h| h.to_str().ok())
            != Some(s.origin.as_str())
    {
        return AppError::Forbidden.into_response();
    }
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("referrer-policy", "same-origin".parse().unwrap());
    headers.insert("content-security-policy","default-src 'self'; script-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'".parse().unwrap());
    response
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Invalid(_) => StatusCode::BAD_REQUEST,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Html(ui::error(&self.to_string()))).into_response()
    }
}
type Fields = HashMap<String, String>;
fn value<'a>(f: &'a Fields, k: &str) -> &'a str {
    f.get(k).map(String::as_str).unwrap_or("")
}
fn token(headers: &HeaderMap) -> AppResult<&str> {
    headers
        .get(header::COOKIE)
        .and_then(|c| c.to_str().ok())
        .and_then(|c| {
            c.split(';')
                .find_map(|part| part.trim().strip_prefix("hph_session="))
        })
        .ok_or(AppError::Unauthorized)
}
async fn session(s: &WebState, h: &HeaderMap) -> AppResult<Session> {
    auth::session(&s.db, token(h)?).await
}
async fn mutation(s: &WebState, h: &HeaderMap, f: &Fields) -> AppResult<Session> {
    let session = session(s, h).await?;
    if value(f, "csrf") != session.csrf {
        return Err(AppError::Forbidden);
    }
    Ok(session)
}
fn cookie(s: &WebState, value: &str, age: u32) -> String {
    format!(
        "hph_session={value}; Path=/; HttpOnly; SameSite=Strict; Max-Age={age}{}",
        if s.secure { "; Secure" } else { "" }
    )
}
async fn login(
    State(s): State<Arc<WebState>>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    Form(f): Form<Fields>,
) -> AppResult<Response> {
    let _slot = s
        .login_slots
        .try_acquire()
        .map_err(|_| AppError::Invalid("Too many login attempts. Try again later."))?;
    let ip = peer
        .map(|p| p.0.0.ip())
        .unwrap_or(std::net::Ipv4Addr::LOCALHOST.into());
    {
        let mut attempts = s.attempts.lock().map_err(|_| AppError::Internal)?;
        attempts.retain(|_, (at, _)| at.elapsed() < Duration::from_secs(900));
        if attempts.len() >= 10000 && !attempts.contains_key(&ip) {
            return Err(AppError::Invalid("Try again later."));
        }
        let entry = attempts.entry(ip).or_insert((Instant::now(), 0));
        entry.1 += 1;
        if entry.1 > 20 {
            return Err(AppError::Invalid(
                "Too many login attempts. Wait 15 minutes.",
            ));
        }
    }
    let token = auth::login(
        &s.db,
        value(&f, "email"),
        value(&f, "password").into(),
        &s.dummy,
    )
    .await?;
    // A successful sign-in rotates any previous browser session.
    if let Ok(old) = self::token(&headers) {
        sqlx::query("DELETE FROM hph_sessions WHERE token_hash=$1")
            .bind(auth::digest(old))
            .execute(s.db.pool())
            .await?;
    }
    let mut response = Redirect::to("/companies").into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie(&s, &token, 28800)
            .parse()
            .map_err(|_| AppError::Internal)?,
    );
    Ok(response)
}
async fn logout(
    State(s): State<Arc<WebState>>,
    h: HeaderMap,
    Form(f): Form<Fields>,
) -> AppResult<Response> {
    let session = mutation(&s, &h, &f).await?;
    sqlx::query("DELETE FROM hph_sessions WHERE token_hash=$1")
        .bind(session.hash)
        .execute(s.db.pool())
        .await?;
    let mut response = Redirect::to("/login").into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, cookie(&s, "", 0).parse().unwrap());
    Ok(response)
}
async fn password(
    State(s): State<Arc<WebState>>,
    h: HeaderMap,
    Form(f): Form<Fields>,
) -> AppResult<Response> {
    let session = mutation(&s, &h, &f).await?;
    let _slot = s
        .login_slots
        .try_acquire()
        .map_err(|_| AppError::Invalid("Please try again shortly."))?;
    let mut tx = s.db.pool().begin().await?;
    let old: String =
        sqlx::query_scalar("SELECT password_hash FROM hph_credentials WHERE user_id=$1 FOR UPDATE")
            .bind(session.user)
            .fetch_one(&mut *tx)
            .await?;
    if !auth::verify(value(&f, "current_password").into(), old).await {
        return Err(AppError::Unauthorized);
    }
    let hash = auth::password_hash(value(&f, "new_password").into()).await?;
    sqlx::query("UPDATE hph_credentials SET password_hash=$1,updated_at=now() WHERE user_id=$2")
        .bind(hash)
        .bind(session.user)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM hph_sessions WHERE user_id=$1")
        .bind(session.user)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Redirect::to("/login").into_response())
}
async fn companies(State(s): State<Arc<WebState>>, h: HeaderMap) -> AppResult<Response> {
    let session = match session(&s, &h).await {
        Ok(s) => s,
        Err(AppError::Unauthorized) => return Ok(Redirect::to("/login").into_response()),
        Err(e) => return Err(e),
    };
    Ok(Html(ui::companies(
        &session,
        &op::companies(&s.db, &session).await?,
    ))
    .into_response())
}
async fn select_company(
    State(s): State<Arc<WebState>>,
    h: HeaderMap,
    Form(f): Form<Fields>,
) -> AppResult<Redirect> {
    let session = mutation(&s, &h, &f).await?;
    op::select_company(&s.db, &session, op::uuid(value(&f, "company"))?).await?;
    Ok(Redirect::to("/app"))
}
async fn workspace(
    State(s): State<Arc<WebState>>,
    h: HeaderMap,
    Query(q): Query<Fields>,
) -> AppResult<Response> {
    let session = match session(&s, &h).await {
        Ok(s) => s,
        Err(AppError::Unauthorized) => return Ok(Redirect::to("/login").into_response()),
        Err(e) => return Err(e),
    };
    if session.tenant.is_none() {
        return Ok(Redirect::to("/companies").into_response());
    }
    let data = op::snapshot(&s.db, &session).await?;
    Ok(Html(ui::workspace(&session, &data, &q)?).into_response())
}
async fn save_person(
    State(s): State<Arc<WebState>>,
    h: HeaderMap,
    Form(f): Form<Fields>,
) -> AppResult<Redirect> {
    let session = mutation(&s, &h, &f).await?;
    op::save_person(
        &s.db,
        &session,
        op::PersonInput {
            id: if value(&f, "id").is_empty() {
                None
            } else {
                Some(op::uuid(value(&f, "id"))?)
            },
            number: value(&f, "number").into(),
            name: value(&f, "name").into(),
            email: value(&f, "email").into(),
            department: value(&f, "department").into(),
            job: value(&f, "job").into(),
            active: value(&f, "active") == "on",
            role: value(&f, "role").into(),
            password: value(&f, "password").into(),
        },
    )
    .await?;
    Ok(Redirect::to("/app?tab=people&saved=1"))
}
async fn punch(
    State(s): State<Arc<WebState>>,
    h: HeaderMap,
    Form(f): Form<Fields>,
) -> AppResult<Redirect> {
    let session = mutation(&s, &h, &f).await?;
    op::punch(
        &s.db,
        &session,
        op::kind(value(&f, "kind"))?,
        op::uuid(value(&f, "key"))?,
    )
    .await?;
    Ok(Redirect::to("/app?tab=attendance&saved=1"))
}
async fn correct(
    State(s): State<Arc<WebState>>,
    h: HeaderMap,
    Form(f): Form<Fields>,
) -> AppResult<Redirect> {
    let session = mutation(&s, &h, &f).await?;
    // Require an explicit offset: ambiguous autumn DST times are never guessed.
    let at = chrono::DateTime::parse_from_rfc3339(value(&f, "at"))
        .map_err(|_| AppError::Invalid("Enter an ISO timestamp with an explicit timezone offset."))?
        .timestamp();
    op::correct(
        &s.db,
        &session,
        op::uuid(value(&f, "event"))?,
        at,
        value(&f, "reason"),
    )
    .await?;
    Ok(Redirect::to("/app?tab=attendance&saved=1"))
}
async fn settings(
    State(s): State<Arc<WebState>>,
    h: HeaderMap,
    Form(f): Form<Fields>,
) -> AppResult<Redirect> {
    let session = mutation(&s, &h, &f).await?;
    op::settings(
        &s.db,
        &session,
        op::Company {
            name: value(&f, "name").into(),
            timezone: value(&f, "timezone").into(),
            theme: value(&f, "theme").into(),
            logo: value(&f, "logo").into(),
            accent: value(&f, "accent").into(),
        },
    )
    .await?;
    Ok(Redirect::to("/app?tab=settings&saved=1"))
}
async fn modules(
    State(s): State<Arc<WebState>>,
    h: HeaderMap,
    Form(f): Form<Fields>,
) -> AppResult<Redirect> {
    let session = mutation(&s, &h, &f).await?;
    let names = ["people", "attendance", "reports", "notes"]
        .into_iter()
        .filter(|n| value(&f, n) == "on")
        .map(str::to_owned)
        .collect();
    op::set_modules(&s.db, &session, names).await?;
    Ok(Redirect::to("/app?tab=settings&saved=1"))
}
async fn note(
    State(s): State<Arc<WebState>>,
    h: HeaderMap,
    Form(f): Form<Fields>,
) -> AppResult<Redirect> {
    let session = mutation(&s, &h, &f).await?;
    op::add_note(&s.db, &session, value(&f, "body")).await?;
    Ok(Redirect::to("/app?tab=notes&saved=1"))
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;

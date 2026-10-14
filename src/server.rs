use hph_core::*;
use leptos::prelude::*;
use serde::Deserialize;
use std::sync::{Arc, Mutex};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub organization: String,
    pub theme: String,
    pub preset: String,
    /// Optional explicit module selection; when present it overrides the named preset.
    pub modules: Option<Vec<ModuleId>>,
}

pub struct DemoState {
    pub workspace: Mutex<Workspace>,
    pub actor: Actor,
    pub database: Option<crate::database::Database>,
}
impl DemoState {
    pub fn new(config: Config) -> Result<Self, String> {
        if config.organization.trim().is_empty() || config.organization.len() > 100 {
            return Err("Invalid company name.".into());
        }
        if !["sage", "indigo"].contains(&config.theme.as_str()) {
            return Err("Invalid theme: use sage or indigo.".into());
        }
        let modules = match config.modules {
            Some(enabled) => ModuleSet::from_enabled(enabled)?,
            None => ModuleSet::preset(&config.preset)?,
        };
        if modules.contains(ModuleId::Notes) {
            return Err("Team notes requires the authenticated workspace.".into());
        }
        let people = [
            (
                "elena",
                "Elena Conti",
                "EC",
                "People & Culture",
                "People manager",
            ),
            ("marco", "Marco Riva", "MR", "Design", "Product designer"),
            (
                "sofia",
                "Sofia Bianchi",
                "SB",
                "Engineering",
                "Software engineer",
            ),
            (
                "luca",
                "Luca Moretti",
                "LM",
                "Operations",
                "Operations specialist",
            ),
        ]
        .into_iter()
        .map(|(id, name, initials, department, job)| Person {
            id: id.into(),
            name: name.into(),
            initials: initials.into(),
            department: department.into(),
            job: job.into(),
        })
        .collect();
        let identity = UserIdentity {
            id: "demo-user".into(),
        };
        let memberships = [Membership {
            user_id: identity.id.clone(),
            tenant_id: "demo".into(),
            person_id: "elena".into(),
            role: Role::Admin,
            active: true,
        }];
        let actor = identity.actor_for("demo", &memberships)?;

        Ok(Self {
            actor,
            database: None,
            workspace: Mutex::new(Workspace {
                tenant_id: "demo".into(),
                organization: config.organization,
                theme: config.theme,
                modules,
                people,
                punches: vec![],
            }),
        })
    }
    pub fn with_database(mut self, database: crate::database::Database) -> Self {
        self.database = Some(database);
        self
    }

    pub fn snapshot(&self, workspace: &Workspace) -> Snapshot {
        Snapshot {
            workspace: workspace.clone(),
            actor_id: self.actor.person_id.clone(),
            catalog: catalog()
                .into_iter()
                .filter(|module| module.id != ModuleId::Notes)
                .collect(),
            server_time: chrono::Utc::now()
                .format("%Y-%m-%d · %H:%M UTC")
                .to_string(),
        }
    }
}
pub fn state() -> Result<Arc<DemoState>, ServerFnError> {
    use_context::<Arc<DemoState>>()
        .ok_or_else(|| ServerFnError::new("Company context is unavailable."))
}

pub fn router(options: LeptosOptions, state: Arc<DemoState>) -> axum::Router {
    use crate::app::{App, shell};
    use axum::{Router, routing::get};
    use leptos_axum::{LeptosRoutes, generate_route_list};
    let health_database = state.database.clone();
    Router::new()
        .route(
            "/health",
            get(move || {
                let database = health_database.clone();
                async move {
                    use axum::http::StatusCode;
                    match database {
                        Some(database) => match database.ping().await {
                            Ok(()) => (StatusCode::OK, "HPH demo ready; PostgreSQL reachable"),
                            Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "HPH database unavailable"),
                        },
                        None => (StatusCode::OK, "HPH demo ready; data is in memory"),
                    }
                }
            }),
        )
        .leptos_routes_with_context(
            &options,
            generate_route_list(App),
            move || provide_context(state.clone()),
            {
                let options = options.clone();
                move || shell(options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use leptos::server_fn::ServerFn;
    use tower::ServiceExt;

    fn app() -> axum::Router {
        let options = get_configuration(None).unwrap().leptos_options;
        router(
            options,
            Arc::new(
                DemoState::new(Config {
                    organization: "Test Company".into(),
                    theme: "sage".into(),
                    preset: "team".into(),
                    modules: None,
                })
                .unwrap(),
            ),
        )
    }

    async fn post(app: axum::Router, path: &str, body: &str) -> (StatusCode, String) {
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .header("accept", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    #[test]
    fn explicit_company_modules_override_preset_and_invalid_lists_fail() {
        let config: Config = toml::from_str(
            r#"
                organization = "Custom Company"
                theme = "sage"
                preset = "essential"
                modules = ["people", "attendance", "reports"]
            "#,
        )
        .unwrap();
        let state = DemoState::new(config).unwrap();
        assert!(
            state
                .workspace
                .lock()
                .unwrap()
                .modules
                .contains(ModuleId::Reports)
        );

        let invalid: Config = toml::from_str(
            r#"
                organization = "Invalid Company"
                theme = "sage"
                preset = "complete"
                modules = ["reports"]
            "#,
        )
        .unwrap();
        assert!(DemoState::new(invalid).is_err());
    }

    #[tokio::test]
    async fn health_endpoint_reports_the_in_memory_demo_mode() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            String::from_utf8(bytes.to_vec()).unwrap(),
            "HPH demo ready; data is in memory"
        );
    }

    #[tokio::test]
    async fn server_functions_share_state_and_reject_double_entry() {
        let app = app();
        let (status, body) = post(app.clone(), crate::api::GetSnapshot::PATH, "").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let snapshot: Snapshot = serde_json::from_str(&body).unwrap();
        assert_eq!(snapshot.workspace.organization, "Test Company");
        let (status, body) = post(app.clone(), crate::api::RegisterPunch::PATH, "kind=in").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, _) = post(app.clone(), crate::api::RegisterPunch::PATH, "kind=in").await;
        assert!(!status.is_success());
        let (_, body) = post(app.clone(), crate::api::GetSnapshot::PATH, "").await;
        let snapshot: Snapshot = serde_json::from_str(&body).unwrap();
        assert_eq!(snapshot.workspace.punches.len(), 1);
        assert_eq!(snapshot.workspace.state("elena"), WorkState::Working);
    }

    #[tokio::test]
    async fn module_enforcement_happens_at_the_http_boundary() {
        let app = app();
        let (status, body) = post(
            app.clone(),
            crate::api::ConfigureModule::PATH,
            "id=attendance&enabled=false",
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, _) = post(app.clone(), crate::api::RegisterPunch::PATH, "kind=in").await;
        assert!(!status.is_success());
        let (status, _) = post(
            app.clone(),
            crate::api::ConfigureModule::PATH,
            "id=reports&enabled=true",
        )
        .await;
        assert!(!status.is_success());
        let (status, _) = post(
            app,
            crate::api::ConfigureModule::PATH,
            "id=unknown&enabled=true",
        )
        .await;
        assert!(!status.is_success());
    }

    #[tokio::test]
    async fn ssr_page_contains_the_workspace_and_demo_disclosure() {
        let response = app()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(html.contains("lang=\"en\""));
        assert!(html.contains("Test Company"));
        assert!(html.contains("LOCAL DEMO"));
        assert!(html.contains("Everything in its place."));
        assert!(html.contains("/pkg/hph.css"));
    }
}

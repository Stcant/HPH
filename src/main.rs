#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use hph::{
        database::Database,
        server::{Config, DemoState, router},
    };
    use leptos::prelude::*;
    use std::sync::Arc;

    if std::env::args().any(|arg| arg == "--migrate") {
        let db = Database::connect(&std::env::var("HPH_MIGRATION_DATABASE_URL")?).await?;
        db.migrate().await?;
        db.close().await;
        println!("Migrations applied.");
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--bootstrap") {
        let db = Database::connect(&std::env::var("HPH_MIGRATION_DATABASE_URL")?).await?;
        db.migrate().await?;
        hph::auth::bootstrap(
            &db,
            std::env::var("HPH_BOOTSTRAP_EMAIL")?,
            std::env::var("HPH_BOOTSTRAP_PASSWORD")?,
            std::env::var("HPH_BOOTSTRAP_NAME")?,
            std::env::var("HPH_BOOTSTRAP_COMPANY")?,
            std::env::var("HPH_BOOTSTRAP_SLUG")?,
        )
        .await?;
        println!("Company and administrator created. Sign in to select the workspace.");
        db.close().await;
        return Ok(());
    }
    if std::env::var("HPH_DEMO").as_deref() != Ok("true") {
        let database = Database::connect(&std::env::var("HPH_DATABASE_URL")?).await?;
        database.verify_runtime_role().await?;
        let origin = std::env::var("HPH_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:3000".into());
        let state = hph::workspace_web::WebState::new(database, origin).await?;
        let address: std::net::SocketAddr = std::env::var("HPH_BIND")
            .unwrap_or_else(|_| "127.0.0.1:3000".into())
            .parse()?;
        if !state.secure && !address.ip().is_loopback() {
            return Err("HTTP development mode must bind to loopback. Configure an HTTPS origin behind a TLS proxy for other deployments.".into());
        }
        let app = hph::workspace_web::router(state);
        let listener = tokio::net::TcpListener::bind(address).await?;
        println!("HPH workspace · listening on {address}");
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
        return Ok(());
    }
    let options = get_configuration(None)?.leptos_options;
    let address = options.site_addr;
    if !address.ip().is_loopback() {
        return Err("The unauthenticated demo can only listen on a loopback address.".into());
    }
    let config: Config = toml::from_str(&std::fs::read_to_string("hph.toml")?)?;
    let state = DemoState::new(config)?;

    let runtime_database_url = std::env::var("HPH_DATABASE_URL").ok();
    let migration_database_url = std::env::var("HPH_MIGRATION_DATABASE_URL").ok();
    if migration_database_url.is_some() && runtime_database_url.is_none() {
        return Err("HPH_MIGRATION_DATABASE_URL requires HPH_DATABASE_URL.".into());
    }
    if let Some(database_url) = migration_database_url {
        let migration_database = Database::connect(&database_url).await?;
        migration_database.migrate().await?;
        migration_database.close().await;
    }
    let database = match runtime_database_url {
        Some(database_url) => Some(Database::connect(&database_url).await?),
        None => None,
    };

    let state = Arc::new(match database {
        Some(database) => state.with_database(database),
        None => state,
    });
    let app = router(options, state);
    println!("HPH LOCAL DEMO · http://{address} · synthetic data; resets on restart");
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

#[cfg(not(feature = "ssr"))]
fn main() {}

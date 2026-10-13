#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use hph::{
        database::Database,
        server::{Config, DemoState, router},
    };
    use leptos::prelude::*;
    use std::sync::Arc;

    if std::env::var("HPH_DEMO").as_deref() != Ok("true") {
        return Err("Questa versione è una demo locale. Imposta HPH_DEMO=true; autenticazione e database di produzione non sono ancora implementati.".into());
    }
    let options = get_configuration(None)?.leptos_options;
    let address = options.site_addr;
    if !address.ip().is_loopback() {
        return Err("La demo senza autenticazione può ascoltare solo su loopback.".into());
    }
    let config: Config = toml::from_str(&std::fs::read_to_string("hph.toml")?)?;
    let state = DemoState::new(config)?;

    let runtime_database_url = std::env::var("HPH_DATABASE_URL").ok();
    let migration_database_url = std::env::var("HPH_MIGRATION_DATABASE_URL").ok();
    if migration_database_url.is_some() && runtime_database_url.is_none() {
        return Err("HPH_MIGRATION_DATABASE_URL richiede HPH_DATABASE_URL.".into());
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

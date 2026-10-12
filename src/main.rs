#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use hph::server::{Config, DemoState, router};
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
    let state = Arc::new(DemoState::new(config)?);
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

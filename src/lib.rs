pub mod api;
pub mod app;

#[cfg(feature = "ssr")]
pub mod database;

#[cfg(feature = "ssr")]
pub mod membership_repository;

#[cfg(feature = "ssr")]
pub mod module_repository;

#[cfg(feature = "ssr")]
pub mod server;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}

#[cfg(feature = "ssr")]
pub mod auth;
#[cfg(feature = "ssr")]
pub mod operations;
#[cfg(feature = "ssr")]
pub mod workspace_ui;
#[cfg(feature = "ssr")]
pub mod workspace_web;

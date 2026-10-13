use hph_core::{ModuleId, PunchKind, Snapshot};
use leptos::prelude::*;

#[server]
pub async fn get_snapshot() -> Result<Snapshot, ServerFnError> {
    let state = crate::server::state()?;
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| ServerFnError::new("State is temporarily unavailable."))?;
    workspace
        .authorize(&state.actor)
        .map_err(ServerFnError::new)?;
    Ok(state.snapshot(&workspace))
}

#[server]
pub async fn configure_module(id: ModuleId, enabled: bool) -> Result<(), ServerFnError> {
    let state = crate::server::state()?;
    let mut workspace = state
        .workspace
        .lock()
        .map_err(|_| ServerFnError::new("State is temporarily unavailable."))?;
    workspace
        .set_module(&state.actor, id, enabled)
        .map_err(ServerFnError::new)
}

#[server]
pub async fn register_punch(kind: PunchKind) -> Result<(), ServerFnError> {
    let state = crate::server::state()?;
    let now = chrono::Utc::now();
    let mut workspace = state
        .workspace
        .lock()
        .map_err(|_| ServerFnError::new("State is temporarily unavailable."))?;
    workspace
        .punch(
            &state.actor,
            kind,
            now.timestamp(),
            now.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        )
        .map_err(ServerFnError::new)
}

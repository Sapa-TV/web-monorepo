//! Thin HTTP layer for session/admin/root endpoints (`/api`).
//!
//! Boundary: handlers extract params -> call a service -> map to a DTO -> return a status.
//! No business logic (`if`/`match`/`for`/`loop`) is allowed here; enforced by the ast-grep
//! rule `.sg/rules/no-control-flow-in-api.yml`. Exceptions (allowlisted): `auth.rs` (middleware).
#![allow(clippy::exhaustive_structs)]

pub mod admin;
pub mod auth;
pub mod session;
pub mod stream;

use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::get;

use crate::api::auth::require_admin;
use crate::api::auth::require_root;
use crate::api::auth::require_session;
use crate::state::AppState;

fn public_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .merge(stream::public_router())
        .merge(session::public_router())
}

async fn health() -> &'static str {
    "ok"
}

async fn version() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "git_sha": option_env!("GIT_SHA").unwrap_or("unknown"),
    }))
}

pub fn router(state: AppState) -> Router {
    let session_layer = from_fn_with_state(state.clone(), require_session);
    let admin_layer = from_fn_with_state(state.clone(), require_admin);
    let root_layer = from_fn_with_state(state.clone(), require_root);

    let admin_protected = admin::session_router().route_layer(admin_layer);
    let root_protected = admin::root_router().route_layer(root_layer);
    let session_protected = session::session_router()
        .merge(admin_protected)
        .merge(root_protected)
        .route_layer(session_layer);

    Router::new()
        .nest("/api", public_router().merge(session_protected))
        .with_state(state)
}

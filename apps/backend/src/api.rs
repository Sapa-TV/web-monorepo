#![allow(clippy::exhaustive_structs)]

pub mod admin;
pub mod auth;
pub mod queue;
pub mod rarities;
pub mod roulette_slots;
pub mod session;
pub mod stream;
pub mod users;
pub mod ws;

use axum::middleware::from_fn_with_state;
use axum::routing::get;
use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::OpenApi as OpenApiSchema;

use crate::api::auth::require_admin;
use crate::api::auth::require_auth;
use crate::api::auth::require_root;
use crate::api::auth::require_session;
use crate::state::AppState;

struct MergeSubdocs;

impl Modify for MergeSubdocs {
    fn modify(&self, openapi: &mut OpenApiSchema) {
        openapi.merge(rarities::RaritiesApiDoc::openapi());
        openapi.merge(roulette_slots::SlotsApiDoc::openapi());
        openapi.merge(users::UsersApiDoc::openapi());
        openapi.merge(queue::QueueApiDoc::openapi());
        openapi.merge(stream::StreamApiDoc::openapi());
        openapi.merge(admin::twitch::AdminTwitchApiDoc::openapi());
        openapi.merge(admin::ingress::AdminIngressApiDoc::openapi());
        openapi.merge(admin::AdminApiDoc::openapi());
        openapi.merge(session::SessionApiDoc::openapi());
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = concat!(env!("CARGO_PKG_NAME"), " API"),
        version = env!("CARGO_PKG_VERSION")
    ),
    tags(
        (name = "slots", description = "Roulette slot management"),
        (name = "rarities", description = "Rarity management"),
        (name = "roulette", description = "Roulette gameplay"),
        (name = "users", description = "User management"),
        (name = "queue", description = "Spin queue"),
        (name = "stream", description = "Stream status"),
        (name = "auth", description = "Sessions and login"),
        (name = "admin", description = "Administrative endpoints")
    ),
    modifiers(&MergeSubdocs)
)]
#[non_exhaustive]
pub struct ApiDoc;

pub fn public_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .merge(ws::public_router())
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

pub fn protected_router() -> axum::Router<AppState> {
    axum::Router::new()
        .merge(rarities::protected_router())
        .merge(roulette_slots::protected_router())
        .merge(users::protected_router())
        .merge(queue::protected_router())
        .merge(stream::protected_router())
}

pub fn session_router() -> axum::Router<AppState> {
    axum::Router::new()
        .merge(session::session_router())
        .merge(admin::session_router())
}

pub fn root_router() -> axum::Router<AppState> {
    admin::root_router()
}

#[cfg(test)]
pub fn router(state: AppState) -> axum::Router {
    public_router()
        .merge(protected_router())
        .merge(session_router())
        .merge(root_router())
        .with_state(state)
}

pub fn router_with_auth(state: AppState) -> axum::Router {
    let key_layer = from_fn_with_state(state.clone(), require_auth);
    let session_layer = from_fn_with_state(state.clone(), require_session);
    let admin_layer = from_fn_with_state(state.clone(), require_admin);
    let root_layer = from_fn_with_state(state.clone(), require_root);

    let key_protected = protected_router().route_layer(key_layer);
    let root_protected = root_router().route_layer(root_layer);
    let admin_protected = admin::session_router().route_layer(admin_layer);
    let session_protected = session::session_router()
        .merge(admin_protected)
        .merge(root_protected)
        .route_layer(session_layer);

    public_router()
        .merge(key_protected)
        .merge(session_protected)
        .with_state(state)
}

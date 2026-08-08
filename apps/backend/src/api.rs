#![allow(clippy::exhaustive_structs)]

pub mod admin;
pub mod auth;
pub mod queue;
pub mod rarities;
pub mod roulette_slots;
pub mod stream;
pub mod users;
pub mod ws;

use axum::routing::get;
use utoipa::openapi::OpenApi as OpenApiSchema;
use utoipa::Modify;
use utoipa::OpenApi;

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
        .merge(admin::protected_router())
}

#[cfg(test)]
pub fn router(state: AppState) -> axum::Router {
    public_router().merge(protected_router()).with_state(state)
}

pub fn router_with_auth(
    state: AppState,
    apply_auth: impl FnOnce(axum::Router<AppState>) -> axum::Router<AppState>,
) -> axum::Router {
    let protected = apply_auth(protected_router());
    public_router().merge(protected).with_state(state)
}

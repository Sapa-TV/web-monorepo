pub mod auth;
pub mod queue;
pub mod rarities;
pub mod roulette_slots;
pub mod users;
pub mod ws;

use utoipa::OpenApi;

use crate::state::AppState;

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
        (name = "queue", description = "Spin queue")
    ),
    nest((path = "/", api = rarities::RaritiesApiDoc)),
    nest((path = "/", api = roulette_slots::SlotsApiDoc)),
    nest((path = "/", api = users::UsersApiDoc)),
    nest((path = "/", api = queue::QueueApiDoc))
)]
#[non_exhaustive]
pub struct ApiDoc;

pub fn public_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/health", axum::routing::get(health))
        .route("/version", axum::routing::get(version))
        .merge(ws::router())
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
        .merge(rarities::router())
        .merge(roulette_slots::router())
        .merge(users::router())
        .merge(queue::router())
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

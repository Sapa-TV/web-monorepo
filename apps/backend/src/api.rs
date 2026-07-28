pub mod roulette_slots;
pub mod users;

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
        (name = "users", description = "User management")
    ),
    nest((path = "/", api = roulette_slots::SlotsApiDoc)),
    nest((path = "/", api = users::UsersApiDoc))
)]
pub struct ApiDoc;

pub fn router() -> axum::Router<AppState> {
    use axum::routing::{delete, get, patch, post, put};

    axum::Router::new()
        .route("/api/slots", get(roulette_slots::list_slots))
        .route("/api/slots", post(roulette_slots::create_slot))
        .route("/api/slots/{id}", put(roulette_slots::update_slot))
        .route("/api/slots/{id}", delete(roulette_slots::delete_slot))
        .route("/api/users", post(users::create_user))
        .route("/api/users", get(users::find_user))
        .route("/api/users/{id}", get(users::get_user))
        .route("/api/users/{id}", patch(users::update_user))
        .route("/api/users/{id}", delete(users::delete_user))
        .route("/api/users/{id}/platforms", post(users::link_platform))
        .route(
            "/api/users/{id}/platforms/{platform}",
            patch(users::update_platform_username),
        )
        .route(
            "/api/users/{id}/platforms/{platform}",
            delete(users::delete_platform),
        )
        .route("/api/platforms", get(users::list_platforms))
}
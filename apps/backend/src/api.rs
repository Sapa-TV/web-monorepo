pub mod roulette_slots;

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
        (name = "roulette", description = "Roulette gameplay")
    ),
    nest((path = "/", api = roulette_slots::SlotsApiDoc))
)]
pub struct ApiDoc;

pub fn router() -> axum::Router<AppState> {
    use axum::routing::{delete, get, post, put};

    axum::Router::new()
        .route("/api/slots", get(roulette_slots::list_slots))
        .route("/api/slots", post(roulette_slots::create_slot))
        .route("/api/slots/{id}", put(roulette_slots::update_slot))
        .route("/api/slots/{id}", delete(roulette_slots::delete_slot))
}
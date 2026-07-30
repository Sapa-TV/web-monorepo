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

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .merge(rarities::router())
        .merge(roulette_slots::router())
        .merge(users::router())
        .merge(queue::router())
        .merge(ws::router())
}

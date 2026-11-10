#![deny(clippy::exhaustive_structs)]
#![deny(clippy::new_ret_no_self)]

pub mod api;
pub mod dto;
pub mod error;
pub mod widget_api;

#[cfg(test)]
pub mod test_fixtures;
#[cfg(test)]
pub(crate) mod usecases;

use composition::state::AppState;
use utoipa::OpenApi;
use utoipa::openapi::OpenApi as OpenApiSpec;
use utoipa_axum::router::OpenApiRouter;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "backend API",
        version = env!("CARGO_PKG_VERSION")
    ),
    tags(
        (name = "slots", description = "Roulette slot management"),
        (name = "rarities", description = "Rarity management"),
        (name = "roulette", description = "Roulette gameplay"),
        (name = "users", description = "User management"),
        (name = "queue", description = "Spin queue"),
        (name = "stream", description = "Stream status"),
        (name = "orders", description = "Game/movie order directory"),
        (name = "auth", description = "Sessions and login"),
        (name = "push", description = "Web push subscriptions"),
        (name = "admin", description = "Administrative endpoints"),
        (name = "system", description = "Service health and version")
    )
)]
#[non_exhaustive]
pub struct ApiDoc;

/// Full spec assembled from the router tree; builds without an AppState instance.
pub fn openapi() -> OpenApiSpec {
    OpenApiRouter::<AppState>::with_openapi(ApiDoc::openapi())
        .merge(widget_api::openapi_router())
        .merge(api::openapi_router())
        .into_openapi()
}

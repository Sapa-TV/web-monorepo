//! Thin HTTP layer for WAK-key-protected widget endpoints (`/wapi`).
//!
//! Boundary: handlers extract params -> call a service -> map to a DTO -> return a status.
//! No business logic (`if`/`match`/`for`/`loop`) is allowed here; enforced by the ast-grep
//! rule `.sg/rules/no-control-flow-in-api.yml`. Exceptions (allowlisted):
//! `auth.rs` (middleware), `ws.rs` (websocket protocol).
#![allow(clippy::exhaustive_structs)]

pub mod auth;
pub mod queue;
pub mod rarities;
pub mod roulette_slots;
pub mod stream;
pub mod users;
pub mod ws;

use axum::Router;
use axum::middleware::from_fn_with_state;
use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;
use crate::widget_api::auth::require_key;

fn base_routes() -> OpenApiRouter<AppState> {
    queue::router()
        .merge(rarities::router())
        .merge(roulette_slots::router())
        .merge(users::router())
        .merge(stream::router())
}

fn assemble(inner: OpenApiRouter<AppState>) -> OpenApiRouter<AppState> {
    OpenApiRouter::<AppState>::new().nest("/wapi", inner)
}

pub fn openapi_router() -> OpenApiRouter<AppState> {
    assemble(
        OpenApiRouter::<AppState>::new()
            .merge(ws::public_router().into())
            .merge(base_routes()),
    )
}

pub fn router(state: AppState) -> Router {
    let key = from_fn_with_state(state.clone(), require_key);
    let inner = OpenApiRouter::<AppState>::new()
        .merge(ws::public_router().into())
        .merge(base_routes().route_layer(key));

    assemble(inner).with_state(state).into()
}

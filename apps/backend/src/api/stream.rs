use axum::Json;
use axum::extract::State;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::stream::StreamStatusResponse;

#[utoipa::path(
    get,
    path = "/stream/status",
    tag = "stream",
    responses(
        (status = 200, description = "Current stream status", body = StreamStatusResponse),
    )
)]
pub async fn get_stream_status(State(state): State<AppState>) -> Json<StreamStatusResponse> {
    Json(StreamStatusResponse::new(state.stream_status.is_online()))
}

pub fn public_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_stream_status))
}

#[cfg(test)]
#[path = "stream.test.rs"]
mod tests;

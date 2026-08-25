use axum::Json;
use axum::extract::State;
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::stream::StreamStatusResponse;

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct SetStreamStatusRequest {
    pub online: bool,
    #[serde(skip)]
    _sealed: (),
}

#[utoipa::path(
    post,
    path = "/stream/status",
    tag = "stream",
    request_body = SetStreamStatusRequest,
    responses(
        (status = 200, description = "Stream status updated", body = StreamStatusResponse),
    )
)]
pub async fn set_stream_status(
    State(state): State<AppState>,
    Json(body): Json<SetStreamStatusRequest>,
) -> Json<StreamStatusResponse> {
    state.stream_status.set_online(body.online);
    Json(StreamStatusResponse::new(body.online))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(set_stream_status))
}

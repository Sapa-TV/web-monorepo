use axum::Json;
use axum::extract::State;
use serde::Deserialize;
use utoipa::OpenApi;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct SetStreamStatusRequest {
    pub online: bool,
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[non_exhaustive]
pub struct StreamStatusResponse {
    pub online: bool,
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
    Json(StreamStatusResponse {
        online: body.online,
    })
}

#[derive(OpenApi)]
#[openapi(
    paths(set_stream_status),
    components(schemas(SetStreamStatusRequest, StreamStatusResponse,))
)]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) struct StreamApiDoc;

pub fn router() -> axum::Router<AppState> {
    use axum::routing::post;
    axum::Router::new().route("/stream/status", post(set_stream_status))
}

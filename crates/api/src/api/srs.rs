use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use composition::srs::{SrsHookPayload, handle_hook};
use composition::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct SrsHookQuery {
    #[serde(default)]
    pub token: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[non_exhaustive]
pub struct SrsHookResponse {
    pub code: i32,
    pub message: String,
}

#[utoipa::path(
    post,
    path = "/srs/hooks",
    tag = "stream",
    params(("token" = String, Query, description = "Shared secret configured in SRS")),
    request_body = SrsHookPayload,
    responses(
        (status = 200, description = "Hook accepted", body = SrsHookResponse),
        (status = 401, description = "Invalid token", body = SrsHookResponse),
    )
)]
pub async fn srs_hook(
    State(state): State<AppState>,
    Query(query): Query<SrsHookQuery>,
    Json(payload): Json<SrsHookPayload>,
) -> Response {
    let outcome = handle_hook(
        &state.stream_status,
        state.config.srs_hook_secret(),
        &query.token,
        &payload,
    );
    let status = StatusCode::from_u16(outcome.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = SrsHookResponse {
        code: outcome.code,
        message: outcome.message.to_string(),
    };
    (status, Json(body)).into_response()
}

pub fn public_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(srs_hook))
}

#[cfg(test)]
#[path = "srs.test.rs"]
mod tests;

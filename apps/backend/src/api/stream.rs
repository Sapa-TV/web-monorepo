use axum::Json;
use axum::extract::State;
use serde::Deserialize;
use utoipa::OpenApi;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Debug, serde::Serialize, ToSchema)]
#[non_exhaustive]
pub struct StreamStatusResponse {
    pub online: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct SetStreamStatusRequest {
    pub online: bool,
}

#[utoipa::path(
    get,
    path = "/api/stream/status",
    tag = "stream",
    responses(
        (status = 200, description = "Current stream status", body = StreamStatusResponse),
    )
)]
pub async fn get_stream_status(State(state): State<AppState>) -> Json<StreamStatusResponse> {
    Json(StreamStatusResponse {
        online: state.stream_status.is_online(),
    })
}

#[utoipa::path(
    post,
    path = "/api/stream/status",
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
    paths(get_stream_status, set_stream_status),
    components(schemas(StreamStatusResponse, SetStreamStatusRequest,))
)]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) struct StreamApiDoc;

pub fn public_router() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new().route("/api/stream/status", get(get_stream_status))
}

pub fn protected_router() -> axum::Router<AppState> {
    use axum::routing::post;
    axum::Router::new().route("/api/stream/status", post(set_stream_status))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::body::to_bytes;
    use axum::http::Request;
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::api::router;
    use crate::test_fixtures::test_state;

    #[tokio::test]
    async fn get_stream_status_offline_by_default() {
        let state = test_state().await;
        let app = router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/stream/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["online"], false);
    }

    #[tokio::test]
    async fn get_stream_status_returns_set_value() {
        let state = test_state().await;
        state.stream_status.set_online(true);
        let app = router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/stream/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["online"], true);
    }
}

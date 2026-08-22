use axum::Json;
use axum::extract::State;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;

#[derive(Debug, serde::Serialize, ToSchema)]
#[non_exhaustive]
pub struct StreamStatusResponse {
    pub online: bool,
}

#[utoipa::path(
    get,
    path = "/stream/status",
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

pub fn public_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_stream_status))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::body::to_bytes;
    use axum::http::Request;
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::test_fixtures::{api_path, test_router, test_state};

    #[tokio::test]
    async fn get_stream_status_offline_by_default() {
        let state = test_state().await;
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/stream/status"))
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
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/stream/status"))
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

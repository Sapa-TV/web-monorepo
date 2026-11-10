use axum::Json;
use axum::extract::{Extension, State};
use axum::http::{HeaderMap, StatusCode, header};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::error::api::ApiError;
use composition::state::AppState;
use model::session::Session;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct PushConfigResponse {
    pub enabled: bool,
    pub public_key: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct PushKeys {
    pub p256dh: String,
    pub auth: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct PushSubscriptionRequest {
    pub endpoint: String,
    pub keys: PushKeys,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct DeletePushSubscriptionRequest {
    pub endpoint: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct PushSubscriptionResponse {
    pub id: u32,
    pub endpoint: String,
    #[serde(skip)]
    _sealed: (),
}

#[utoipa::path(
    get,
    path = "/push/config",
    tag = "push",
    responses((status = 200, description = "Web push configuration", body = PushConfigResponse))
)]
pub async fn get_push_config(State(state): State<AppState>) -> Json<PushConfigResponse> {
    Json(PushConfigResponse {
        enabled: state.config.push_enabled(),
        public_key: state.config.vapid_public_key().map(str::to_string),
        _sealed: (),
    })
}

#[utoipa::path(
    post,
    path = "/push/subscriptions",
    tag = "push",
    request_body = PushSubscriptionRequest,
    responses(
        (status = 201, description = "Subscription stored", body = PushSubscriptionResponse),
        (status = 400, description = "Invalid subscription"),
    )
)]
pub async fn subscribe(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
    headers: HeaderMap,
    Json(body): Json<PushSubscriptionRequest>,
) -> Result<(StatusCode, Json<PushSubscriptionResponse>), ApiError> {
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok());
    let subscription = state
        .push_service
        .subscribe(
            session.user_id,
            &body.endpoint,
            &body.keys.p256dh,
            &body.keys.auth,
            user_agent,
        )
        .await?;
    let response = PushSubscriptionResponse {
        id: subscription.id.get(),
        endpoint: subscription.endpoint,
        _sealed: (),
    };
    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    delete,
    path = "/push/subscriptions",
    tag = "push",
    request_body = DeletePushSubscriptionRequest,
    responses((status = 204, description = "Subscription removed"))
)]
pub async fn unsubscribe(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
    Json(body): Json<DeletePushSubscriptionRequest>,
) -> Result<StatusCode, ApiError> {
    state
        .push_service
        .unsubscribe(session.user_id, &body.endpoint)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn public_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_push_config))
}

pub fn session_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(subscribe, unsubscribe))
}

#[cfg(test)]
#[path = "push.test.rs"]
mod tests;

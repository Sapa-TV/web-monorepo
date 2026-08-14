use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::error::QueueServiceError;
use crate::error::api::ApiError;
use crate::queue::entry::{QueueEntry, QueueEntryId, QueueStats, QueueStatus};
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
use crate::state::AppState;
use crate::user::UserId;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct EnqueueRequest {
    pub user_id: UserId,
    pub user_name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct AnonymousEnqueueRequest {
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct QueueEntryResponse {
    pub id: QueueEntryId,
    pub user_id: UserId,
    pub user_name: String,
    pub status: QueueStatus,
    pub result_slot_id: Option<RouletteSlotId>,
    pub slot_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&QueueEntry> for QueueEntryResponse {
    fn from(entry: &QueueEntry) -> Self {
        Self {
            id: entry.id,
            user_id: entry.user_id,
            user_name: entry.user_name.clone(),
            status: entry.status,
            result_slot_id: entry.result_slot_id,
            slot_name: None,
            created_at: entry.created_at.to_rfc3339(),
            updated_at: entry.updated_at.to_rfc3339(),
        }
    }
}

impl QueueEntryResponse {
    fn with_slot_name(mut self, state: &AppState) -> Self {
        self.slot_name = self
            .result_slot_id
            .and_then(|id| state.slot_service.get_name(id));
        self
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct NextResponse {
    pub entry: QueueEntryResponse,
    pub slot: RouletteSlot,
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct QueueListResponse {
    pub entries: Vec<QueueEntryResponse>,
    pub next_cursor: Option<QueueEntryId>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct ListQuery {
    pub status: Option<QueueStatus>,
    pub limit: Option<usize>,
    pub cursor: Option<QueueEntryId>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct QueueIdParam {
    pub id: QueueEntryId,
}

#[utoipa::path(
    post,
    path = "/queue",
    tag = "queue",
    request_body = EnqueueRequest,
    responses(
        (status = 200, description = "Entry enqueued", body = QueueEntryResponse),
    )
)]
pub async fn enqueue(
    State(state): State<AppState>,
    Json(body): Json<EnqueueRequest>,
) -> Result<(StatusCode, Json<QueueEntryResponse>), ApiError> {
    let entry = state
        .queue_service
        .enqueue(body.user_id, &body.user_name)
        .await?;
    let resp = QueueEntryResponse::from(&entry);
    Ok((StatusCode::OK, Json(resp)))
}

#[utoipa::path(
    post,
    path = "/queue/anonymous",
    tag = "queue",
    request_body = AnonymousEnqueueRequest,
    responses(
        (status = 200, description = "Entry enqueued", body = QueueEntryResponse),
    )
)]
pub async fn enqueue_anonymous(
    State(state): State<AppState>,
    Json(body): Json<AnonymousEnqueueRequest>,
) -> Result<(StatusCode, Json<QueueEntryResponse>), ApiError> {
    let guest_id = state.user_service.guest_user_id().await?;
    let entry = state.queue_service.enqueue(guest_id, &body.name).await?;
    let resp = QueueEntryResponse::from(&entry);
    Ok((StatusCode::OK, Json(resp)))
}

#[utoipa::path(
    get,
    path = "/queue",
    tag = "queue",
    params(ListQuery),
    responses(
        (status = 200, description = "Page of queue entries", body = QueueListResponse),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<QueueListResponse>, ApiError> {
    let limit = query.limit.unwrap_or(state.config.queue_default_limit());
    let page = state
        .queue_service
        .list(query.status, query.cursor, limit)
        .await?;
    let entries = page
        .entries
        .iter()
        .map(|entry| QueueEntryResponse::from(entry).with_slot_name(&state))
        .collect();
    Ok(Json(QueueListResponse {
        entries,
        next_cursor: page.next_cursor,
    }))
}

#[utoipa::path(
    get,
    path = "/queue/{id}",
    tag = "queue",
    params(QueueIdParam),
    responses(
        (status = 200, description = "Queue entry found", body = QueueEntryResponse),
        (status = 404, description = "Queue entry not found"),
    )
)]
pub async fn get_by_id(
    State(state): State<AppState>,
    Path(params): Path<QueueIdParam>,
) -> Result<Json<QueueEntryResponse>, ApiError> {
    let entry = state
        .queue_service
        .get_by_id(params.id)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "queue entry not found"))?;
    Ok(Json(
        QueueEntryResponse::from(&entry).with_slot_name(&state),
    ))
}

#[utoipa::path(
    get,
    path = "/queue/next",
    tag = "queue",
    responses(
        (status = 200, description = "Next entry", body = QueueEntryResponse),
        (status = 404, description = "No pending or error entries"),
    )
)]
pub async fn peek_next(
    State(state): State<AppState>,
) -> Result<Json<QueueEntryResponse>, ApiError> {
    let entry = state
        .queue_service
        .peek_next()
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "no pending or error entries"))?;
    Ok(Json(
        QueueEntryResponse::from(&entry).with_slot_name(&state),
    ))
}

#[utoipa::path(
    post,
    path = "/queue/next",
    tag = "queue",
    responses(
        (status = 200, description = "Spin started", body = NextResponse),
        (status = 404, description = "No pending or error entries"),
        (status = 409, description = "A spin is already active"),
        (status = 422, description = "No slots configured"),
    )
)]
pub async fn dequeue_next(
    State(state): State<AppState>,
) -> Result<Json<NextResponse>, QueueServiceError> {
    let (entry, slot) = state.queue_service.dequeue_next().await?;
    Ok(Json(NextResponse {
        entry: QueueEntryResponse::from(&entry).with_slot_name(&state),
        slot,
    }))
}

#[utoipa::path(
    post,
    path = "/queue/{id}/complete",
    tag = "queue",
    params(QueueIdParam),
    responses(
        (status = 200, description = "Spin completed"),
        (status = 404, description = "Queue entry not found"),
        (status = 409, description = "Entry is not in spinning state"),
    )
)]
pub async fn complete(
    State(state): State<AppState>,
    Path(params): Path<QueueIdParam>,
) -> Result<StatusCode, QueueServiceError> {
    state.queue_service.complete(params.id).await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/queue/{id}/cancel",
    tag = "queue",
    params(QueueIdParam),
    responses(
        (status = 200, description = "Entry cancelled"),
        (status = 404, description = "Queue entry not found"),
        (status = 409, description = "Only pending or error entries can be cancelled"),
    )
)]
pub async fn cancel(
    State(state): State<AppState>,
    Path(params): Path<QueueIdParam>,
) -> Result<StatusCode, QueueServiceError> {
    state.queue_service.cancel(params.id).await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/queue/stats",
    tag = "queue",
    responses(
        (status = 200, description = "Queue statistics", body = QueueStats),
    )
)]
pub async fn stats(State(state): State<AppState>) -> Result<Json<QueueStats>, ApiError> {
    let stats = state.queue_service.count_by_status().await?;
    Ok(Json(stats))
}

#[derive(OpenApi)]
#[openapi(
    paths(
        enqueue,
        enqueue_anonymous,
        list,
        get_by_id,
        peek_next,
        dequeue_next,
        complete,
        cancel,
        stats,
    ),
    components(schemas(
        QueueEntryResponse,
        QueueListResponse,
        QueueStats,
        QueueEntryId,
        QueueStatus,
        EnqueueRequest,
        AnonymousEnqueueRequest,
        NextResponse,
        UserId,
        RouletteSlotId,
    ))
)]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) struct QueueApiDoc;

pub fn router() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/queue", post(enqueue))
        .route("/queue/anonymous", post(enqueue_anonymous))
        .route("/queue", get(list))
        .route("/queue/{id}", get(get_by_id))
        .route("/queue/next", get(peek_next))
        .route("/queue/next", post(dequeue_next))
        .route("/queue/{id}/complete", post(complete))
        .route("/queue/{id}/cancel", post(cancel))
        .route("/queue/stats", get(stats))
}

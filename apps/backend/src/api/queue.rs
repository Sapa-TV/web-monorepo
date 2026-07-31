use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::error::QueueServiceError;
use crate::error::RepositoryError;
use crate::error::api::ApiError;
use crate::queue::entry::{QueueEntry, QueueEntryId, QueueStats, QueueStatus};
use crate::queue::repository::QueueRepository;
use crate::roulette::repository::RouletteSlotRepository;
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
use crate::state::AppState;
use crate::user::UserId;
use crate::user::repository::UserRepository;

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
            created_at: entry.created_at.and_utc().to_rfc3339(),
            updated_at: entry.updated_at.and_utc().to_rfc3339(),
        }
    }
}

async fn resolve_slot_name(
    slot_id: Option<crate::roulette::slot_service::RouletteSlotId>,
    slot_repo: &impl RouletteSlotRepository,
) -> Result<Option<String>, RepositoryError> {
    let Some(slot_id) = slot_id else {
        return Ok(None);
    };
    let slots = slot_repo.load_all().await?;
    Ok(slots.into_iter().find(|s| s.id == slot_id).map(|s| s.name))
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct NextResponse {
    pub entry: QueueEntryResponse,
    pub slot: RouletteSlot,
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct ListQuery {
    pub status: Option<QueueStatus>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct QueueIdParam {
    pub id: QueueEntryId,
}

#[cfg(test)]
mod tests {
    use tower::ServiceExt;

    use crate::api::queue::EnqueueRequest;
    use crate::api::router;
    use crate::queue::entry::{QueueEntryId, QueueStatus};
    use crate::queue::repository::QueueRepository;
    use crate::roulette::rarity::{Rarity, RarityId, RarityRepository};
    use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
    use crate::test_fixtures::test_state;
    use crate::user::UserId;
    use crate::user::repository::UserRepository;

    async fn setup_user(state: &crate::state::AppState) -> UserId {
        state.user_repo.create("user1").await.unwrap().id
    }

    fn enqueue_body(user_id: UserId) -> EnqueueRequest {
        EnqueueRequest {
            user_id,
            user_name: "user1".to_string(),
        }
    }

    #[tokio::test]
    async fn dequeue_next_retries_error_entry() {
        let state = test_state().await;

        state
            .rarity_repo
            .save(Rarity::new(
                RarityId::new(1),
                "common",
                "Common",
                "c.png",
                "#fff",
            ))
            .await
            .unwrap();
        state
            .slot_service
            .add_slot(RouletteSlot::new(
                RouletteSlotId::new(0),
                "test_slot",
                RarityId::new(1),
                100,
                "test",
            ))
            .await
            .unwrap();

        let user_id = setup_user(&state).await;
        let app = router(state.clone());

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/queue")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&enqueue_body(user_id)).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/queue/next")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        state
            .queue_repo
            .update_status(QueueEntryId::new(1), QueueStatus::Error, None)
            .await
            .unwrap();

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/queue/next")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn dequeue_next_returns_409_when_already_active() {
        let state = test_state().await;

        state
            .rarity_repo
            .save(Rarity::new(
                RarityId::new(1),
                "common",
                "Common",
                "c.png",
                "#fff",
            ))
            .await
            .unwrap();
        state
            .slot_service
            .add_slot(RouletteSlot::new(
                RouletteSlotId::new(0),
                "test_slot",
                RarityId::new(1),
                100,
                "test",
            ))
            .await
            .unwrap();

        let user_id = setup_user(&state).await;
        let app = router(state.clone());

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/queue")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&enqueue_body(user_id)).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/queue/next")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/queue/next")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), 409);
    }

    #[tokio::test]
    async fn dequeue_next_returns_200() {
        let state = test_state().await;

        state
            .rarity_repo
            .save(Rarity::new(
                RarityId::new(1),
                "common",
                "Common",
                "c.png",
                "#fff",
            ))
            .await
            .unwrap();
        state
            .slot_service
            .add_slot(RouletteSlot::new(
                RouletteSlotId::new(0),
                "test_slot",
                RarityId::new(1),
                100,
                "test",
            ))
            .await
            .unwrap();

        let user_id = setup_user(&state).await;
        let app = router(state.clone());

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/queue")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&enqueue_body(user_id)).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/queue/next")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
    }
}

#[utoipa::path(
    post,
    path = "/api/queue",
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
        .queue_repo
        .enqueue(body.user_id, &body.user_name)
        .await?;
    let resp = QueueEntryResponse::from(&entry);
    Ok((StatusCode::OK, Json(resp)))
}

async fn guest_user_id(state: &AppState) -> Result<UserId, RepositoryError> {
    match state.guest_user_id.get() {
        Some(id) => Ok(*id),
        None => {
            let user = state.user_repo.create("guest").await?;
            Ok(*state.guest_user_id.get_or_init(|| user.id))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/queue/anonymous",
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
    let guest_id = guest_user_id(&state).await?;
    let entry = state.queue_repo.enqueue(guest_id, &body.name).await?;
    let resp = QueueEntryResponse::from(&entry);
    Ok((StatusCode::OK, Json(resp)))
}

#[utoipa::path(
    get,
    path = "/api/queue",
    tag = "queue",
    params(ListQuery),
    responses(
        (status = 200, description = "List of queue entries", body = Vec<QueueEntryResponse>),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<QueueEntryResponse>>, ApiError> {
    let entries = state.queue_repo.list(query.status).await?;
    let slots = state.slot_repo.load_all().await?;
    let mut responses = Vec::with_capacity(entries.len());
    for entry in &entries {
        let mut resp = QueueEntryResponse::from(entry);
        resp.slot_name = entry
            .result_slot_id
            .and_then(|sid| slots.iter().find(|s| s.id == sid))
            .map(|s| s.name.clone());
        responses.push(resp);
    }
    Ok(Json(responses))
}

#[utoipa::path(
    get,
    path = "/api/queue/{id}",
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
        .queue_repo
        .get_by_id(params.id)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "queue entry not found"))?;
    let mut resp = QueueEntryResponse::from(&entry);
    resp.slot_name = resolve_slot_name(entry.result_slot_id, &*state.slot_repo).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/api/queue/next",
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
        .queue_repo
        .peek_next()
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "no pending or error entries"))?;
    let mut resp = QueueEntryResponse::from(&entry);
    resp.slot_name = resolve_slot_name(entry.result_slot_id, &*state.slot_repo).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    post,
    path = "/api/queue/next",
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
    let mut entry_resp = QueueEntryResponse::from(&entry);
    entry_resp.slot_name = Some(slot.name.clone());
    Ok(Json(NextResponse {
        entry: entry_resp,
        slot,
    }))
}

#[utoipa::path(
    post,
    path = "/api/queue/{id}/complete",
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
    path = "/api/queue/{id}/cancel",
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
    path = "/api/queue/stats",
    tag = "queue",
    responses(
        (status = 200, description = "Queue statistics", body = QueueStats),
    )
)]
pub async fn stats(State(state): State<AppState>) -> Result<Json<QueueStats>, ApiError> {
    let stats = state.queue_repo.count_by_status().await?;
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
pub(crate) struct QueueApiDoc;

pub fn router() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/queue", post(enqueue))
        .route("/api/queue/anonymous", post(enqueue_anonymous))
        .route("/api/queue", get(list))
        .route("/api/queue/{id}", get(get_by_id))
        .route("/api/queue/next", get(peek_next))
        .route("/api/queue/next", post(dequeue_next))
        .route("/api/queue/{id}/complete", post(complete))
        .route("/api/queue/{id}/cancel", post(cancel))
        .route("/api/queue/stats", get(stats))
}

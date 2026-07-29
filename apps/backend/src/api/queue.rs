use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::error::QueueServiceError;
use crate::error::api::ApiError;
use crate::platform::PlatformRepository;
use crate::queue::entry::{QueueEntry, QueueEntryId, QueueStats, QueueStatus};
use crate::queue::repository::QueueRepository;
use crate::roulette::slot_service::RouletteSlot;
use crate::state::AppState;
use crate::user::repository::UserRepository;

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct EnqueueRequest {
    pub platform: String,
    pub platform_user_id: String,
    pub platform_username: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct QueueEntryResponse {
    pub id: QueueEntryId,
    pub user_id: u32,
    pub status: QueueStatus,
    pub result_slot_id: Option<u32>,
    pub created_at: String,
    pub updated_at: String,
}

fn to_entry_response(entry: &QueueEntry) -> QueueEntryResponse {
    QueueEntryResponse {
        id: entry.id,
        user_id: entry.user_id.value(),
        status: entry.status,
        result_slot_id: entry.result_slot_id.map(|id| id.value()),
        created_at: entry.created_at.and_utc().to_rfc3339(),
        updated_at: entry.updated_at.and_utc().to_rfc3339(),
    }
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
    use axum::Router;
    use tower::ServiceExt;

    use crate::api::router;
    use crate::queue::entry::{QueueEntryId, QueueStatus};
    use crate::queue::repository::QueueRepository;
    use crate::roulette::rarity::{Rarity, RarityId, RarityRepository};
    use crate::roulette::repository::RouletteSlotRepository;
    use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
    use crate::test_fixtures::test_state;

    #[tokio::test]
    async fn dequeue_next_retries_error_entry() {
        let state = test_state();

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
            .slot_repo
            .save(RouletteSlot::new(
                RouletteSlotId::new(0),
                "test_slot",
                RarityId::new(1),
                100,
                "test",
            ))
            .await
            .unwrap();

        let app = Router::new().merge(router()).with_state(state.clone());

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/queue")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        r#"{"platform":"twitch","platform_user_id":"u1","platform_username":"user1"}"#,
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
        let state = test_state();

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
            .slot_repo
            .save(RouletteSlot::new(
                RouletteSlotId::new(0),
                "test_slot",
                RarityId::new(1),
                100,
                "test",
            ))
            .await
            .unwrap();

        let app = Router::new().merge(router()).with_state(state.clone());

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/queue")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        r#"{"platform":"twitch","platform_user_id":"u1","platform_username":"user1"}"#,
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
        let state = test_state();

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
            .slot_repo
            .save(RouletteSlot::new(
                RouletteSlotId::new(0),
                "test_slot",
                RarityId::new(1),
                100,
                "test",
            ))
            .await
            .unwrap();

        let app = Router::new().merge(router()).with_state(state.clone());

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/queue")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        r#"{"platform":"twitch","platform_user_id":"u1","platform_username":"user1"}"#,
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

async fn resolve_platform(
    name: &str,
    platform_repo: &impl PlatformRepository,
) -> Result<crate::platform::Platform, ApiError> {
    platform_repo
        .find_by_name(name)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, format!("unknown platform: {name}")))
}

#[utoipa::path(
    post,
    path = "/api/queue",
    tag = "queue",
    request_body = EnqueueRequest,
    responses(
        (status = 200, description = "Entry enqueued", body = QueueEntryResponse),
        (status = 400, description = "Unknown platform"),
    )
)]
pub async fn enqueue(
    State(state): State<AppState>,
    Json(body): Json<EnqueueRequest>,
) -> Result<(StatusCode, Json<QueueEntryResponse>), ApiError> {
    let platform = resolve_platform(&body.platform, &*state.platform_repo).await?;

    let user = match state
        .user_repo
        .find_by_platform(platform.id, &body.platform_user_id)
        .await?
    {
        Some(user) => user,
        None => {
            let user = state.user_repo.create(&body.platform_username).await?;
            state
                .user_repo
                .link_platform(
                    user.id,
                    platform.id,
                    &body.platform_user_id,
                    &body.platform_username,
                )
                .await?;
            user
        }
    };

    let entry = state.queue_repo.enqueue(user.id).await?;
    Ok((StatusCode::OK, Json(to_entry_response(&entry))))
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
    Ok(Json(entries.iter().map(to_entry_response).collect()))
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
    Ok(Json(to_entry_response(&entry)))
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
    Ok(Json(to_entry_response(&entry)))
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
    Ok(Json(NextResponse {
        entry: to_entry_response(&entry),
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
        NextResponse,
    ))
)]
#[non_exhaustive]
pub(crate) struct QueueApiDoc;

pub fn router() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/queue", post(enqueue))
        .route("/api/queue", get(list))
        .route("/api/queue/{id}", get(get_by_id))
        .route("/api/queue/next", get(peek_next))
        .route("/api/queue/next", post(dequeue_next))
        .route("/api/queue/{id}/complete", post(complete))
        .route("/api/queue/{id}/cancel", post(cancel))
        .route("/api/queue/stats", get(stats))
}

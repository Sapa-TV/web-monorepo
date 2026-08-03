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

#[cfg(test)]
mod tests {
    use tower::ServiceExt;

    use crate::api::queue::EnqueueRequest;
    use crate::api::router;
    use crate::roulette::rarity::{Rarity, RarityId};
    use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
    use crate::test_fixtures::test_state;
    use crate::user::UserId;

    async fn setup_user(state: &crate::state::AppState) -> UserId {
        state.user_service.create("user1").await.unwrap().id
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
            .rarity_service
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

        state.queue_service.mark_timed_out().await.unwrap();

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
            .rarity_service
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
            .rarity_service
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

    #[tokio::test]
    async fn dequeue_next_parallel_only_one_spin() {
        let state = test_state().await;

        state
            .rarity_service
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

        let user_1 = setup_user(&state).await;
        let user_2 = setup_user(&state).await;
        let app = router(state.clone());

        for user in [user_1, user_2] {
            let resp = app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("POST")
                        .uri("/api/queue")
                        .header("content-type", "application/json")
                        .body(axum::body::Body::from(
                            serde_json::to_string(&enqueue_body(user)).unwrap(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), 200);
        }

        let make_next = || {
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/queue/next")
                .body(axum::body::Body::empty())
                .unwrap()
        };

        let (a, b) = tokio::join!(app.clone().oneshot(make_next()), app.oneshot(make_next()));
        let statuses = [a.unwrap().status(), b.unwrap().status()];
        assert_eq!(statuses.iter().filter(|s| **s == 200).count(), 1);
        assert_eq!(statuses.iter().filter(|s| **s == 409).count(), 1);
    }

    #[tokio::test]
    async fn dequeue_next_no_slots_no_orphan() {
        let state = test_state().await;
        let app = router(state.clone());

        let user_id = setup_user(&state).await;
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
        assert_eq!(resp.status(), 422);

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/queue?status=Spinning")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["entries"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn complete_parallel_only_one_success() {
        let state = test_state().await;

        state
            .rarity_service
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
        let body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        let entry_id = body["id"].as_u64().unwrap();

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

        let complete = |id: u64| {
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/api/queue/{id}/complete"))
                .body(axum::body::Body::empty())
                .unwrap()
        };

        let (a, b) = tokio::join!(
            app.clone().oneshot(complete(entry_id)),
            app.oneshot(complete(entry_id)),
        );
        let statuses = [a.unwrap().status(), b.unwrap().status()];
        assert_eq!(statuses.iter().filter(|s| **s == 200).count(), 1);
        assert_eq!(statuses.iter().filter(|s| **s == 409).count(), 1);
    }

    #[tokio::test]
    async fn slot_created_via_api_used_in_roll() {
        let state = test_state().await;
        let app = router(state.clone());

        state
            .rarity_service
            .save(Rarity::new(
                RarityId::new(1),
                "common",
                "Common",
                "c.png",
                "#fff",
            ))
            .await
            .unwrap();

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/slots")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        r#"{"name":"api_slot","rarity_id":1,"weight":100,"action":"act"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 201);

        let user_id = setup_user(&state).await;
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
        let body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["slot"]["name"], "api_slot");
    }

    #[tokio::test]
    async fn list_status_query_is_case_insensitive() {
        let state = test_state().await;
        let app = router(state.clone());

        let user_id = setup_user(&state).await;
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

        for status in ["pending", "Pending", "spinning", "Spinning"] {
            let resp = app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("GET")
                        .uri(format!("/api/queue?status={status}"))
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                resp.status(),
                200,
                "status query {status} should be accepted"
            );
        }
    }

    #[tokio::test]
    async fn list_is_paginated_with_cursor() {
        let state = test_state().await;
        let app = router(state.clone());

        let user_id = setup_user(&state).await;
        for _ in 0..3 {
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
        }

        let first = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/queue?limit=2")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(first.status(), 200);
        let first_body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(first.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        let entries = first_body["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        let cursor = first_body["next_cursor"].as_u64().unwrap();
        assert_eq!(cursor, 2);

        let second = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri(format!("/api/queue?limit=2&cursor={cursor}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(second.status(), 200);
        let second_body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(second.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(second_body["entries"].as_array().unwrap().len(), 1);
        assert!(second_body["next_cursor"].is_null());
    }

    #[tokio::test]
    async fn enqueue_anonymous_reuses_single_guest() {
        let state = test_state().await;
        let app = router(state.clone());

        let enqueue = |app: axum::Router, name: String| async move {
            app.oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/queue/anonymous")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(format!(r#"{{"name":"{name}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap()
        };

        let first = enqueue(app.clone(), "viewer1".to_string()).await;
        assert_eq!(first.status(), 200);
        let first_body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(first.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();

        let second = enqueue(app, "viewer2".to_string()).await;
        assert_eq!(second.status(), 200);
        let second_body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(second.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(first_body["user_id"], second_body["user_id"]);
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
        .queue_service
        .enqueue(body.user_id, &body.user_name)
        .await?;
    let resp = QueueEntryResponse::from(&entry);
    Ok((StatusCode::OK, Json(resp)))
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
    let guest_id = state.user_service.guest_user_id().await?;
    let entry = state.queue_service.enqueue(guest_id, &body.name).await?;
    let resp = QueueEntryResponse::from(&entry);
    Ok((StatusCode::OK, Json(resp)))
}

#[utoipa::path(
    get,
    path = "/api/queue",
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
    let limit = query.limit.unwrap_or(state.config.queue_default_limit);
    let page = state
        .queue_service
        .list(query.status, query.cursor, limit)
        .await?;
    let entries = page
        .entries
        .iter()
        .map(|entry| {
            let mut resp = QueueEntryResponse::from(entry);
            resp.slot_name = entry
                .result_slot_id
                .and_then(|id| state.slot_service.get_name(id));
            resp
        })
        .collect();
    Ok(Json(QueueListResponse {
        entries,
        next_cursor: page.next_cursor,
    }))
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
        .queue_service
        .get_by_id(params.id)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "queue entry not found"))?;
    let mut resp = QueueEntryResponse::from(&entry);
    resp.slot_name = entry
        .result_slot_id
        .and_then(|id| state.slot_service.get_name(id));
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
        .queue_service
        .peek_next()
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "no pending or error entries"))?;
    let mut resp = QueueEntryResponse::from(&entry);
    resp.slot_name = entry
        .result_slot_id
        .and_then(|id| state.slot_service.get_name(id));
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

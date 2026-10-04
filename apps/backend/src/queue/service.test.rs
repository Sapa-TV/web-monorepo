use chrono::Utc;

use crate::error::QueueServiceError;
use crate::queue::entry::QueueStatus;
use crate::roulette::rarity::{Rarity, RarityId};
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
use crate::test_fixtures::{InMemoryAppState, test_state_inmemory};
use crate::user::UserId;

use super::*;

async fn setup_slots(state: &InMemoryAppState) -> UserId {
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
    state.user_service.create("user1").await.unwrap().id
}

#[tokio::test]
async fn dequeue_next_returns_200() {
    let (state, _queue_repo) = test_state_inmemory().await;
    let user_id = setup_slots(&state).await;
    state.queue_service.enqueue(user_id, "user1").await.unwrap();

    let (entry, slot) = state.queue_service.dequeue_next().await.unwrap();
    assert_eq!(entry.status, QueueStatus::Spinning);
    assert_eq!(slot.name, "test_slot");
    assert_eq!(entry.result_slot_id, Some(slot.id));
}

#[tokio::test]
async fn dequeue_next_returns_409_when_already_active() {
    let (state, _queue_repo) = test_state_inmemory().await;
    let user_id = setup_slots(&state).await;
    state.queue_service.enqueue(user_id, "user1").await.unwrap();
    state.queue_service.dequeue_next().await.unwrap();

    let err = state.queue_service.dequeue_next().await.unwrap_err();
    assert!(matches!(err, QueueServiceError::AlreadyActive));
}

#[tokio::test]
async fn dequeue_next_parallel_only_one_spin() {
    let (state, _queue_repo) = test_state_inmemory().await;
    let user_1 = setup_slots(&state).await;
    let user_2 = state.user_service.create("user2").await.unwrap().id;
    state.queue_service.enqueue(user_1, "user1").await.unwrap();
    state.queue_service.enqueue(user_2, "user2").await.unwrap();

    let (a, b) = tokio::join!(
        state.queue_service.dequeue_next(),
        state.queue_service.dequeue_next(),
    );
    let results = [a, b];
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(QueueServiceError::AlreadyActive)))
            .count(),
        1
    );
}

#[tokio::test]
async fn dequeue_next_retries_error_entry() {
    let (state, queue_repo) = test_state_inmemory().await;
    let user_id = setup_slots(&state).await;
    state.queue_service.enqueue(user_id, "user1").await.unwrap();

    let (first, _) = state.queue_service.dequeue_next().await.unwrap();
    queue_repo.mark_timed_out(Utc::now()).await.unwrap();
    let (second, _) = state.queue_service.dequeue_next().await.unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(second.status, QueueStatus::Spinning);
}

#[tokio::test]
async fn dequeue_next_no_slots_no_orphan() {
    let (state, _queue_repo) = test_state_inmemory().await;
    let user_id = state.user_service.create("user1").await.unwrap().id;
    state.queue_service.enqueue(user_id, "user1").await.unwrap();

    let err = state.queue_service.dequeue_next().await.unwrap_err();
    assert!(matches!(err, QueueServiceError::NoSlots));

    let page = state
        .queue_service
        .list(Some(QueueStatus::Spinning), None, 100)
        .await
        .unwrap();
    assert!(page.entries.is_empty());
}

#[tokio::test]
async fn complete_parallel_only_one_success() {
    let (state, _queue_repo) = test_state_inmemory().await;
    let user_id = setup_slots(&state).await;
    state.queue_service.enqueue(user_id, "user1").await.unwrap();
    let (entry, _) = state.queue_service.dequeue_next().await.unwrap();

    // ws `complete` and REST `complete` both call `QueueService::complete`,
    // so this exercise covers the shared path for both transports.
    let (a, b) = tokio::join!(
        state.queue_service.complete(entry.id),
        state.queue_service.complete(entry.id),
    );
    let results = [a, b];
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(QueueServiceError::NotSpinning)))
            .count(),
        1
    );
}

#[tokio::test]
async fn list_status_query_is_case_insensitive() {
    let (state, _queue_repo) = test_state_inmemory().await;
    let user_id = setup_slots(&state).await;
    state.queue_service.enqueue(user_id, "user1").await.unwrap();

    for raw in ["pending", "Pending", "spinning", "Spinning"] {
        let status: QueueStatus = serde_json::from_str(&format!("\"{raw}\"")).unwrap();
        let _ = state
            .queue_service
            .list(Some(status), None, 100)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn list_is_paginated_with_cursor() {
    let (state, _queue_repo) = test_state_inmemory().await;
    let user_id = state.user_service.create("user1").await.unwrap().id;
    for _ in 0..3 {
        state.queue_service.enqueue(user_id, "user1").await.unwrap();
    }

    let first = state.queue_service.list(None, None, 2).await.unwrap();
    assert_eq!(first.entries.len(), 2);
    let cursor = first.next_cursor.unwrap();

    let second = state
        .queue_service
        .list(None, Some(cursor), 2)
        .await
        .unwrap();
    assert_eq!(second.entries.len(), 1);
    assert!(second.next_cursor.is_none());
}

#[tokio::test]
async fn enqueue_anonymous_reuses_single_guest() {
    let (state, _queue_repo) = test_state_inmemory().await;

    let guest_1 = state.user_service.guest_user_id().await.unwrap();
    let first = state
        .queue_service
        .enqueue(guest_1, "viewer1")
        .await
        .unwrap();
    let guest_2 = state.user_service.guest_user_id().await.unwrap();
    let second = state
        .queue_service
        .enqueue(guest_2, "viewer2")
        .await
        .unwrap();
    assert_eq!(first.user_id, second.user_id);
}

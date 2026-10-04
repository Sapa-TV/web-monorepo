use crate::roulette::slot_service::RouletteSlotId;
use crate::user::UserId;

use super::*;

async fn repo_with_pending_and_error() -> (InMemoryQueueRepository, QueueEntry) {
    let repo = InMemoryQueueRepository::new();
    let first = repo.enqueue(UserId::new(1), "a").await.unwrap();
    repo.enqueue(UserId::new(2), "b").await.unwrap();
    repo.update_status_if(first.id, QueueStatus::Pending, QueueStatus::Error)
        .await
        .unwrap();
    (repo, first)
}

#[tokio::test]
async fn peek_prefers_error_over_pending() {
    let (repo, error_entry) = repo_with_pending_and_error().await;
    let peeked = repo.peek_next().await.unwrap().unwrap();
    assert_eq!(peeked.id, error_entry.id);
}

#[tokio::test]
async fn dequeue_prefers_error_over_pending() {
    let (repo, error_entry) = repo_with_pending_and_error().await;
    match repo
        .dequeue_next_with_slot(RouletteSlotId::new(0))
        .await
        .unwrap()
    {
        DequeueOutcome::Picked(entry) => assert_eq!(entry.id, error_entry.id),
        _ => panic!("expected Picked"),
    }
}

#[tokio::test]
async fn list_is_paginated_by_keyset_cursor() {
    let repo = InMemoryQueueRepository::new();
    for i in 0..5 {
        repo.enqueue(UserId::new(i as u32 + 1), &format!("u{i}"))
            .await
            .unwrap();
    }

    let first = repo.list(None, None, 2).await.unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(first[0].id, QueueEntryId::new(1));
    assert_eq!(first[1].id, QueueEntryId::new(2));

    let second = repo.list(None, Some(first[1].id), 2).await.unwrap();
    assert_eq!(second.len(), 2);
    assert_eq!(second[0].id, QueueEntryId::new(3));
    assert_eq!(second[1].id, QueueEntryId::new(4));

    let third = repo.list(None, Some(second[1].id), 2).await.unwrap();
    assert_eq!(third.len(), 1);
    assert_eq!(third[0].id, QueueEntryId::new(5));
}

#[tokio::test]
async fn list_filters_by_status() {
    let repo = InMemoryQueueRepository::new();
    let entry = repo.enqueue(UserId::new(1), "a").await.unwrap();
    repo.update_status_if(entry.id, QueueStatus::Pending, QueueStatus::Completed)
        .await
        .unwrap();

    let pending = repo
        .list(Some(QueueStatus::Pending), None, 100)
        .await
        .unwrap();
    assert!(pending.is_empty());
    let completed = repo
        .list(Some(QueueStatus::Completed), None, 100)
        .await
        .unwrap();
    assert_eq!(completed.len(), 1);
}

#[tokio::test]
async fn purge_removes_only_expired_completed_and_cancelled() {
    let repo = InMemoryQueueRepository::new();
    let done = repo.enqueue(UserId::new(1), "done").await.unwrap();
    let cancelled = repo.enqueue(UserId::new(2), "cancelled").await.unwrap();
    let pending = repo.enqueue(UserId::new(3), "pending").await.unwrap();

    repo.update_status_if(done.id, QueueStatus::Pending, QueueStatus::Completed)
        .await
        .unwrap();
    repo.update_status_if(cancelled.id, QueueStatus::Pending, QueueStatus::Cancelled)
        .await
        .unwrap();

    let future_cutoff = Utc::now() + chrono::Duration::seconds(3600);
    let removed = repo.purge_completed_cancelled(future_cutoff).await.unwrap();
    assert_eq!(removed, 2);

    let remaining = repo.list(None, None, 100).await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, pending.id);
}

#[tokio::test]
async fn purge_skips_fresh_completed() {
    let repo = InMemoryQueueRepository::new();
    let done = repo.enqueue(UserId::new(1), "done").await.unwrap();
    repo.update_status_if(done.id, QueueStatus::Pending, QueueStatus::Completed)
        .await
        .unwrap();

    let past_cutoff = Utc::now() - chrono::Duration::seconds(3600);
    let removed = repo.purge_completed_cancelled(past_cutoff).await.unwrap();
    assert_eq!(removed, 0);
    assert_eq!(repo.list(None, None, 100).await.unwrap().len(), 1);
}

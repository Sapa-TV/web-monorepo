use chrono::{Duration as ChronoDuration, Utc};

use crate::db::sqlite::test_pool;
use crate::roulette::slot_service::RouletteSlotId;
use crate::user::UserId;

use super::*;

async fn repo() -> SqliteQueueRepository {
    let (pool, _path) = test_pool().await;
    SqliteQueueRepository::new(pool)
}

#[tokio::test]
async fn enqueue_returns_pending_entry() {
    let repo = repo().await;

    let entry = repo.enqueue(UserId::new(1), "viewer").await.unwrap();

    assert_eq!(entry.id.get(), 1);
    assert_eq!(entry.status, QueueStatus::Pending);
    assert_eq!(entry.user_name, "viewer");
    assert_eq!(entry.created_at, entry.updated_at);
    assert_eq!(repo.count_by_status().await.unwrap().pending, 1);
}

async fn repo_with_pending_and_error() -> (SqliteQueueRepository, QueueEntry) {
    let repo = repo().await;
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
        DequeueOutcome::Picked(entry) => {
            assert_eq!(entry.id, error_entry.id);
            assert_eq!(entry.status, QueueStatus::Spinning);
        }
        _ => panic!("expected Picked"),
    }
}

#[tokio::test]
async fn dequeue_reports_already_active_and_empty() {
    let repo = repo().await;

    match repo
        .dequeue_next_with_slot(RouletteSlotId::new(1))
        .await
        .unwrap()
    {
        DequeueOutcome::Empty => {}
        _ => panic!("expected Empty"),
    }

    let entry = repo.enqueue(UserId::new(1), "a").await.unwrap();
    repo.update_status_if(entry.id, QueueStatus::Pending, QueueStatus::Spinning)
        .await
        .unwrap();

    match repo
        .dequeue_next_with_slot(RouletteSlotId::new(1))
        .await
        .unwrap()
    {
        DequeueOutcome::AlreadyActive => {}
        _ => panic!("expected AlreadyActive"),
    }
}

#[tokio::test]
async fn list_is_paginated_by_keyset_cursor() {
    let repo = repo().await;
    for i in 0..5u32 {
        repo.enqueue(UserId::new(i + 1), &format!("u{i}"))
            .await
            .unwrap();
    }

    let first = repo.list(None, None, 2).await.unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(first[0].id.get(), 1);
    assert_eq!(first[1].id.get(), 2);

    let second = repo.list(None, Some(first[1].id), 2).await.unwrap();
    assert_eq!(second.len(), 2);
    assert_eq!(second[0].id.get(), 3);

    let third = repo.list(None, Some(second[1].id), 2).await.unwrap();
    assert_eq!(third.len(), 1);
    assert_eq!(third[0].id.get(), 5);
}

#[tokio::test]
async fn list_filters_by_status() {
    let repo = repo().await;
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
async fn update_status_if_outcomes() {
    let repo = repo().await;
    let entry = repo.enqueue(UserId::new(1), "a").await.unwrap();

    let updated = repo
        .update_status_if(entry.id, QueueStatus::Pending, QueueStatus::Spinning)
        .await
        .unwrap();
    match updated {
        StatusUpdateOutcome::Updated(e) => {
            assert_eq!(e.status, QueueStatus::Spinning);
            assert!(e.updated_at >= e.created_at);
        }
        _ => panic!("expected Updated"),
    }

    match repo
        .update_status_if(entry.id, QueueStatus::Pending, QueueStatus::Completed)
        .await
        .unwrap()
    {
        StatusUpdateOutcome::StatusMismatch => {}
        _ => panic!("expected StatusMismatch"),
    }

    match repo
        .update_status_if(
            QueueEntryId::new(999),
            QueueStatus::Pending,
            QueueStatus::Error,
        )
        .await
        .unwrap()
    {
        StatusUpdateOutcome::NotFound => {}
        _ => panic!("expected NotFound"),
    }
}

#[tokio::test]
async fn mark_timed_out_transitions_spinning_to_error_once() {
    let repo = repo().await;
    let spun = repo.enqueue(UserId::new(1), "a").await.unwrap();
    let done = repo.enqueue(UserId::new(2), "b").await.unwrap();
    repo.update_status_if(spun.id, QueueStatus::Pending, QueueStatus::Spinning)
        .await
        .unwrap();
    repo.update_status_if(done.id, QueueStatus::Pending, QueueStatus::Completed)
        .await
        .unwrap();

    let cutoff = Utc::now() + ChronoDuration::hours(1);
    let timed_out = repo.mark_timed_out(cutoff).await.unwrap();

    assert_eq!(timed_out.len(), 1);
    assert_eq!(timed_out[0].id, spun.id);
    assert_eq!(timed_out[0].status, QueueStatus::Error);

    assert!(repo.mark_timed_out(cutoff).await.unwrap().is_empty());
}

#[tokio::test]
async fn purge_removes_only_expired_completed_and_cancelled() {
    let repo = repo().await;
    let done = repo.enqueue(UserId::new(1), "done").await.unwrap();
    let cancelled = repo.enqueue(UserId::new(2), "cancelled").await.unwrap();
    let pending = repo.enqueue(UserId::new(3), "pending").await.unwrap();

    repo.update_status_if(done.id, QueueStatus::Pending, QueueStatus::Completed)
        .await
        .unwrap();
    repo.update_status_if(cancelled.id, QueueStatus::Pending, QueueStatus::Cancelled)
        .await
        .unwrap();

    let future_cutoff = Utc::now() + ChronoDuration::hours(1);
    let removed = repo.purge_completed_cancelled(future_cutoff).await.unwrap();
    assert_eq!(removed, 2);

    let remaining = repo.list(None, None, 100).await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, pending.id);
}

#[tokio::test]
async fn purge_skips_fresh_completed() {
    let repo = repo().await;
    let done = repo.enqueue(UserId::new(1), "done").await.unwrap();
    repo.update_status_if(done.id, QueueStatus::Pending, QueueStatus::Completed)
        .await
        .unwrap();

    let past_cutoff = Utc::now() - ChronoDuration::hours(1);
    let removed = repo.purge_completed_cancelled(past_cutoff).await.unwrap();

    assert_eq!(removed, 0);
    assert_eq!(repo.list(None, None, 100).await.unwrap().len(), 1);
}

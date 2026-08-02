use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::nonpoison::Mutex;

use chrono::{DateTime, Utc};

use crate::error::RepositoryError;
use crate::queue::entry::{QueueEntry, QueueEntryId, QueueStats, QueueStatus};
use crate::queue::repository::{DequeueOutcome, QueueRepository, StatusUpdateOutcome};
use crate::roulette::slot_service::RouletteSlotId;
use crate::user::UserId;

#[non_exhaustive]
pub struct InMemoryQueueRepository {
    entries: Mutex<Vec<QueueEntry>>,
    next_id: AtomicU32,
}

impl InMemoryQueueRepository {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            next_id: AtomicU32::new(1),
        }
    }
}

impl QueueRepository for InMemoryQueueRepository {
    async fn enqueue(
        &self,
        user_id: UserId,
        user_name: &str,
    ) -> Result<QueueEntry, RepositoryError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Utc::now();
        let entry = QueueEntry::new(
            QueueEntryId::new(id),
            user_id,
            user_name,
            QueueStatus::Pending,
            None,
            now,
            now,
        );
        self.entries.lock().push(entry.clone());
        Ok(entry)
    }

    async fn peek_next(&self) -> Result<Option<QueueEntry>, RepositoryError> {
        let entries = self.entries.lock();
        if let Some(error) = entries.iter().find(|e| e.status == QueueStatus::Error) {
            return Ok(Some(error.clone()));
        }
        Ok(entries
            .iter()
            .find(|e| e.status == QueueStatus::Pending)
            .cloned())
    }

    async fn dequeue_next_with_slot(
        &self,
        slot_id: RouletteSlotId,
    ) -> Result<DequeueOutcome, RepositoryError> {
        let mut entries = self.entries.lock();
        if entries.iter().any(|e| e.status == QueueStatus::Spinning) {
            return Ok(DequeueOutcome::AlreadyActive);
        }

        let pos = entries
            .iter()
            .position(|e| e.status == QueueStatus::Error)
            .or_else(|| {
                entries
                    .iter()
                    .position(|e| e.status == QueueStatus::Pending)
            });

        let Some(pos) = pos else {
            return Ok(DequeueOutcome::Empty);
        };

        let entry = &mut entries[pos];
        entry.status = QueueStatus::Spinning;
        entry.result_slot_id = Some(slot_id);
        entry.updated_at = Utc::now();
        Ok(DequeueOutcome::Picked(entry.clone()))
    }

    async fn list(
        &self,
        status: Option<QueueStatus>,
        cursor: Option<QueueEntryId>,
        limit: usize,
    ) -> Result<Vec<QueueEntry>, RepositoryError> {
        let entries = self.entries.lock();
        Ok(entries
            .iter()
            .filter(|e| cursor.is_none_or(|c| e.id > c))
            .filter(|e| status.is_none_or(|s| e.status == s))
            .take(limit)
            .cloned()
            .collect())
    }

    async fn get_by_id(&self, id: QueueEntryId) -> Result<Option<QueueEntry>, RepositoryError> {
        let entries = self.entries.lock();
        Ok(entries.iter().find(|e| e.id == id).cloned())
    }

    async fn update_status_if(
        &self,
        id: QueueEntryId,
        expected: QueueStatus,
        status: QueueStatus,
    ) -> Result<StatusUpdateOutcome, RepositoryError> {
        let mut entries = self.entries.lock();
        let Some(entry) = entries.iter_mut().find(|e| e.id == id) else {
            return Ok(StatusUpdateOutcome::NotFound);
        };
        if entry.status != expected {
            return Ok(StatusUpdateOutcome::StatusMismatch);
        }
        entry.status = status;
        entry.updated_at = Utc::now();
        Ok(StatusUpdateOutcome::Updated(entry.clone()))
    }

    async fn count_by_status(&self) -> Result<QueueStats, RepositoryError> {
        let entries = self.entries.lock();
        let mut stats = QueueStats::new(0, 0, 0, 0, 0);
        for entry in entries.iter() {
            match entry.status {
                QueueStatus::Pending => stats.pending += 1,
                QueueStatus::Spinning => stats.spinning += 1,
                QueueStatus::Completed => stats.completed += 1,
                QueueStatus::Error => stats.error += 1,
                QueueStatus::Cancelled => stats.cancelled += 1,
            }
        }
        Ok(stats)
    }

    async fn mark_timed_out(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<QueueEntry>, RepositoryError> {
        let mut entries = self.entries.lock();
        let now = Utc::now();
        let mut result = Vec::new();
        for entry in entries.iter_mut() {
            if entry.status == QueueStatus::Spinning && entry.updated_at < cutoff {
                entry.status = QueueStatus::Error;
                entry.updated_at = now;
                result.push(entry.clone());
            }
        }
        Ok(result)
    }

    async fn purge_completed_cancelled(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<usize, RepositoryError> {
        let mut entries = self.entries.lock();
        let len_before = entries.len();
        entries.retain(|e| {
            !((e.status == QueueStatus::Completed || e.status == QueueStatus::Cancelled)
                && e.updated_at < cutoff)
        });
        Ok(len_before - entries.len())
    }
}

#[cfg(test)]
mod tests {
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
}

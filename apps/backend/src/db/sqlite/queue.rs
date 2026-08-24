use chrono::{DateTime, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, SqlitePool};

use crate::db::sqlite::map_err;
use crate::error::RepositoryError;
use crate::queue::entry::{QueueEntry, QueueEntryId, QueueStats, QueueStatus};
use crate::queue::repository::{DequeueOutcome, QueueRepository, StatusUpdateOutcome};
use crate::roulette::slot_service::RouletteSlotId;
use crate::user::UserId;

#[non_exhaustive]
pub struct SqliteQueueRepository {
    pool: SqlitePool,
}

impl SqliteQueueRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn status_to_db(status: QueueStatus) -> &'static str {
    <&'static str>::from(&status)
}

fn status_from_db(status: &str) -> Result<QueueStatus, RepositoryError> {
    QueueStatus::try_from(status)
        .map_err(|_| RepositoryError::Database(format!("invalid queue status: {status}")))
}

#[derive(sqlx::FromRow)]
struct EntryRow {
    id: i64,
    user_id: i64,
    user_name: String,
    status: String,
    result_slot_id: Option<i64>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl EntryRow {
    fn into_entry(self) -> Result<QueueEntry, RepositoryError> {
        Ok(QueueEntry::new(
            QueueEntryId::new(self.id as u32),
            UserId::new(self.user_id as u32),
            self.user_name,
            status_from_db(&self.status)?,
            self.result_slot_id.map(|id| RouletteSlotId::new(id as u32)),
            self.created_at,
            self.updated_at,
        ))
    }
}

fn row_to_entry(row: SqliteRow) -> Result<QueueEntry, sqlx::Error> {
    EntryRow::from_row(&row)?
        .into_entry()
        .map_err(|e| sqlx::Error::Configuration(Box::new(e)))
}

impl QueueRepository for SqliteQueueRepository {
    async fn enqueue(
        &self,
        user_id: UserId,
        user_name: &str,
    ) -> Result<QueueEntry, RepositoryError> {
        let now = Utc::now();
        let row = sqlx::query!(
            "INSERT INTO queue_entries (user_id, user_name, status, created_at, updated_at)
             VALUES (?, ?, 'pending', ?, ?)
             RETURNING id AS \"id!: i64\"",
            user_id.get(),
            user_name,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(QueueEntry::new(
            QueueEntryId::new(row.id as u32),
            user_id,
            user_name,
            QueueStatus::Pending,
            None,
            now,
            now,
        ))
    }

    /// Invariant (shared with the in-memory implementation): entries in
    /// Error are preferred over Pending, both by ascending id.
    async fn peek_next(&self) -> Result<Option<QueueEntry>, RepositoryError> {
        let row = sqlx::query_as!(
            EntryRow,
            r#"SELECT id AS "id!: i64", user_id AS "user_id!: i64", user_name, status, result_slot_id, created_at AS "created_at: DateTime<Utc>", updated_at AS "updated_at: DateTime<Utc>"
               FROM queue_entries
               WHERE status IN ('error', 'pending')
               ORDER BY CASE status WHEN 'error' THEN 0 ELSE 1 END, id
               LIMIT 1"#
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        match row {
            Some(row) => Ok(Some(row.into_entry()?)),
            None => Ok(None),
        }
    }

    /// Atomic single-statement dequeue: SQLite serializes writers, so the
    /// "no Spinning" guard inside the subquery cannot race.
    async fn dequeue_next_with_slot(
        &self,
        slot_id: RouletteSlotId,
    ) -> Result<DequeueOutcome, RepositoryError> {
        let picked = sqlx::query_as!(
            EntryRow,
            r#"UPDATE queue_entries
               SET status = 'spinning', result_slot_id = ?1, updated_at = ?2
               WHERE id = (
                   SELECT id FROM queue_entries
                   WHERE status IN ('error', 'pending')
                     AND NOT EXISTS (SELECT 1 FROM queue_entries WHERE status = 'spinning')
                   ORDER BY CASE status WHEN 'error' THEN 0 ELSE 1 END, id
                   LIMIT 1
               )
               RETURNING id AS "id!: i64", user_id AS "user_id!: i64", user_name, status, result_slot_id, created_at AS "created_at: DateTime<Utc>", updated_at AS "updated_at: DateTime<Utc>""#,
            slot_id.get(),
            Utc::now()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        if let Some(row) = picked {
            return Ok(DequeueOutcome::Picked(row.into_entry()?));
        }

        let spinning_exists = sqlx::query!(
            r#"SELECT EXISTS(SELECT 1 FROM queue_entries WHERE status = 'spinning') AS "exists!: bool""#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        if spinning_exists.r#exists {
            Ok(DequeueOutcome::AlreadyActive)
        } else {
            Ok(DequeueOutcome::Empty)
        }
    }

    async fn list(
        &self,
        status: Option<QueueStatus>,
        cursor: Option<QueueEntryId>,
        limit: usize,
    ) -> Result<Vec<QueueEntry>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, user_id, user_name, status, result_slot_id, created_at, updated_at
             FROM queue_entries
             WHERE (?1 IS NULL OR id > ?1) AND (?2 IS NULL OR status = ?2)
             ORDER BY id ASC LIMIT ?3",
        )
        .bind(cursor.map(|c| c.get() as i64))
        .bind(status.map(status_to_db))
        .bind(limit as i64)
        .try_map(row_to_entry)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows)
    }

    async fn get_by_id(&self, id: QueueEntryId) -> Result<Option<QueueEntry>, RepositoryError> {
        let row = sqlx::query_as!(
            EntryRow,
            r#"SELECT id AS "id!: i64", user_id AS "user_id!: i64", user_name, status, result_slot_id, created_at AS "created_at: DateTime<Utc>", updated_at AS "updated_at: DateTime<Utc>"
               FROM queue_entries WHERE id = ?"#,
            id.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        match row {
            Some(row) => Ok(Some(row.into_entry()?)),
            None => Ok(None),
        }
    }

    async fn update_status_if(
        &self,
        id: QueueEntryId,
        expected: QueueStatus,
        status: QueueStatus,
    ) -> Result<StatusUpdateOutcome, RepositoryError> {
        let now = Utc::now();
        let result = sqlx::query!(
            "UPDATE queue_entries SET status = ?1, updated_at = ?2 WHERE id = ?3 AND status = ?4",
            status_to_db(status),
            now,
            id.get(),
            status_to_db(expected)
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        if result.rows_affected() == 0 {
            let exists = sqlx::query!(
                r#"SELECT EXISTS(SELECT 1 FROM queue_entries WHERE id = ?) AS "exists!: bool""#,
                id.get()
            )
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
            return Ok(if exists.r#exists {
                StatusUpdateOutcome::StatusMismatch
            } else {
                StatusUpdateOutcome::NotFound
            });
        }
        match self.get_by_id(id).await? {
            Some(entry) => Ok(StatusUpdateOutcome::Updated(entry)),
            None => Err(RepositoryError::Database(
                "entry vanished after update".into(),
            )),
        }
    }

    async fn count_by_status(&self) -> Result<QueueStats, RepositoryError> {
        let rows = sqlx::query!(
            r#"SELECT status, COUNT(*) AS "count!: i64" FROM queue_entries GROUP BY status"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        let mut stats = QueueStats::new(0, 0, 0, 0, 0);
        for row in rows {
            let count = row.count as u32;
            match status_from_db(&row.status)? {
                QueueStatus::Pending => stats.pending = count,
                QueueStatus::Spinning => stats.spinning = count,
                QueueStatus::Completed => stats.completed = count,
                QueueStatus::Error => stats.error = count,
                QueueStatus::Cancelled => stats.cancelled = count,
            }
        }
        Ok(stats)
    }

    async fn mark_timed_out(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<QueueEntry>, RepositoryError> {
        let rows = sqlx::query_as!(
            EntryRow,
            r#"UPDATE queue_entries SET status = 'error', updated_at = ?1
               WHERE status = 'spinning' AND updated_at < ?2
               RETURNING id AS "id!: i64", user_id AS "user_id!: i64", user_name, status, result_slot_id, created_at AS "created_at: DateTime<Utc>", updated_at AS "updated_at: DateTime<Utc>""#,
            Utc::now(),
            cutoff
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        rows.into_iter().map(|row| row.into_entry()).collect()
    }

    async fn purge_completed_cancelled(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<usize, RepositoryError> {
        let result = sqlx::query!(
            "DELETE FROM queue_entries
             WHERE status IN ('completed', 'cancelled') AND updated_at < ?",
            cutoff
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
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
}

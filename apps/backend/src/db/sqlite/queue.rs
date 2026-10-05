use chrono::{DateTime, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, SqlitePool};

use crate::db::sqlite::map_err;
use crate::error::RepositoryError;
use crate::platform::PlatformId;
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
    platform_id: Option<i64>,
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
        )
        .with_platform(self.platform_id.map(|id| PlatformId::new(id as u32))))
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
            r#"SELECT id AS "id!: i64", user_id AS "user_id!: i64", user_name, status, result_slot_id, created_at AS "created_at: DateTime<Utc>", updated_at AS "updated_at: DateTime<Utc>",
                   (SELECT up.platform_id FROM user_platforms up WHERE up.user_id = queue_entries.user_id ORDER BY up.platform_id LIMIT 1) AS platform_id
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
               RETURNING id AS "id!: i64", user_id AS "user_id!: i64", user_name, status, result_slot_id, created_at AS "created_at: DateTime<Utc>", updated_at AS "updated_at: DateTime<Utc>",
                   (SELECT up.platform_id FROM user_platforms up WHERE up.user_id = queue_entries.user_id ORDER BY up.platform_id LIMIT 1) AS platform_id"#,
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
            "SELECT id, user_id, user_name, status, result_slot_id, created_at, updated_at,
                    (SELECT up.platform_id FROM user_platforms up WHERE up.user_id = queue_entries.user_id ORDER BY up.platform_id LIMIT 1) AS platform_id
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
            r#"SELECT id AS "id!: i64", user_id AS "user_id!: i64", user_name, status, result_slot_id, created_at AS "created_at: DateTime<Utc>", updated_at AS "updated_at: DateTime<Utc>",
                   (SELECT up.platform_id FROM user_platforms up WHERE up.user_id = queue_entries.user_id ORDER BY up.platform_id LIMIT 1) AS platform_id
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
               RETURNING id AS "id!: i64", user_id AS "user_id!: i64", user_name, status, result_slot_id, created_at AS "created_at: DateTime<Utc>", updated_at AS "updated_at: DateTime<Utc>",
                   CAST(NULL AS INTEGER) AS platform_id"#,
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
#[path = "queue.test.rs"]
mod tests;

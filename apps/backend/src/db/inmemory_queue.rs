use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::nonpoison::Mutex;

use chrono::{NaiveDateTime, Utc};

use crate::error::RepositoryError;
use crate::queue::entry::{QueueEntry, QueueEntryId, QueueStats, QueueStatus};
use crate::queue::repository::QueueRepository;
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
        let now = Utc::now().naive_utc();
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

    async fn dequeue_next(&self) -> Result<Option<QueueEntry>, RepositoryError> {
        let mut entries = self.entries.lock();
        let pos = entries
            .iter()
            .position(|e| e.status == QueueStatus::Error)
            .or_else(|| {
                entries
                    .iter()
                    .position(|e| e.status == QueueStatus::Pending)
            });

        if let Some(pos) = pos {
            let entry = &mut entries[pos];
            entry.status = QueueStatus::Spinning;
            entry.updated_at = Utc::now().naive_utc();
            Ok(Some(entry.clone()))
        } else {
            Ok(None)
        }
    }

    async fn list(&self, status: Option<QueueStatus>) -> Result<Vec<QueueEntry>, RepositoryError> {
        let entries = self.entries.lock();
        Ok(match status {
            Some(s) => entries.iter().filter(|e| e.status == s).cloned().collect(),
            None => entries.clone(),
        })
    }

    async fn get_by_id(&self, id: QueueEntryId) -> Result<Option<QueueEntry>, RepositoryError> {
        let entries = self.entries.lock();
        Ok(entries.iter().find(|e| e.id == id).cloned())
    }

    async fn update_status(
        &self,
        id: QueueEntryId,
        status: QueueStatus,
        result_slot_id: Option<RouletteSlotId>,
    ) -> Result<Option<QueueEntry>, RepositoryError> {
        let mut entries = self.entries.lock();
        if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
            entry.status = status;
            entry.result_slot_id = result_slot_id;
            entry.updated_at = Utc::now().naive_utc();
            Ok(Some(entry.clone()))
        } else {
            Ok(None)
        }
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

    async fn find_timed_out(
        &self,
        cutoff: NaiveDateTime,
    ) -> Result<Vec<QueueEntry>, RepositoryError> {
        let entries = self.entries.lock();
        Ok(entries
            .iter()
            .filter(|e| e.status == QueueStatus::Spinning && e.updated_at < cutoff)
            .cloned()
            .collect())
    }

    async fn mark_timed_out(
        &self,
        cutoff: NaiveDateTime,
    ) -> Result<Vec<QueueEntry>, RepositoryError> {
        let mut entries = self.entries.lock();
        let now = Utc::now().naive_utc();
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
}

use std::future::Future;

use chrono::NaiveDateTime;

use crate::error::RepositoryError;
use crate::queue::entry::{QueueEntry, QueueEntryId, QueueStats, QueueStatus};
use crate::roulette::slot_service::RouletteSlotId;

#[non_exhaustive]
pub enum DequeueOutcome {
    Picked(QueueEntry),
    AlreadyActive,
    Empty,
}

#[non_exhaustive]
pub enum StatusUpdateOutcome {
    Updated(QueueEntry),
    NotFound,
    StatusMismatch,
}

pub trait QueueRepository: Send + Sync {
    fn enqueue(
        &self,
        user_id: crate::user::UserId,
        user_name: &str,
    ) -> impl Future<Output = Result<QueueEntry, RepositoryError>> + Send;

    fn peek_next(&self)
    -> impl Future<Output = Result<Option<QueueEntry>, RepositoryError>> + Send;

    fn dequeue_next_with_slot(
        &self,
        slot_id: RouletteSlotId,
    ) -> impl Future<Output = Result<DequeueOutcome, RepositoryError>> + Send;

    fn list(
        &self,
        status: Option<QueueStatus>,
    ) -> impl Future<Output = Result<Vec<QueueEntry>, RepositoryError>> + Send;

    fn get_by_id(
        &self,
        id: QueueEntryId,
    ) -> impl Future<Output = Result<Option<QueueEntry>, RepositoryError>> + Send;

    fn update_status_if(
        &self,
        id: QueueEntryId,
        expected: QueueStatus,
        status: QueueStatus,
    ) -> impl Future<Output = Result<StatusUpdateOutcome, RepositoryError>> + Send;

    fn count_by_status(&self) -> impl Future<Output = Result<QueueStats, RepositoryError>> + Send;

    fn mark_timed_out(
        &self,
        cutoff: NaiveDateTime,
    ) -> impl Future<Output = Result<Vec<QueueEntry>, RepositoryError>> + Send;
}

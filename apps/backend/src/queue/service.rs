use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::error::QueueServiceError;
use crate::event::BroadcastEventPublisher;
use crate::queue::entry::{QueueEntry, QueueEntryId, QueueStats, QueueStatus};
use crate::queue::events::{SpinEvent, SpinEventPublisher};
use crate::queue::repository::{DequeueOutcome, QueueRepository, StatusUpdateOutcome};
use crate::random::StandartRandomProvider;
use crate::roulette::machine::RouletteService;
use crate::roulette::rarity::RarityRepository;
use crate::roulette::rarity_service::RarityService;
use crate::roulette::repository::RouletteSlotRepository;
use crate::roulette::slot_service::RouletteSlot;
use crate::user::UserId;

type Roulette<R> = RouletteService<StandartRandomProvider, Arc<R>>;

#[non_exhaustive]
pub struct QueueService<Q, R, S>
where
    Q: QueueRepository,
    R: RarityRepository,
    S: RouletteSlotRepository,
{
    queue_repo: Arc<Q>,
    rarity_service: Arc<RarityService<Arc<R>>>,
    roulette: Roulette<S>,
    event_publisher: BroadcastEventPublisher,
    timeout: Duration,
}

impl<Q, R, S> Clone for QueueService<Q, R, S>
where
    Q: QueueRepository,
    R: RarityRepository,
    S: RouletteSlotRepository,
{
    fn clone(&self) -> Self {
        Self {
            queue_repo: Arc::clone(&self.queue_repo),
            rarity_service: Arc::clone(&self.rarity_service),
            roulette: self.roulette.clone(),
            event_publisher: self.event_publisher.clone(),
            timeout: self.timeout,
        }
    }
}

impl<Q, R, S> QueueService<Q, R, S>
where
    Q: QueueRepository,
    R: RarityRepository,
    S: RouletteSlotRepository,
{
    pub fn new(
        queue_repo: Arc<Q>,
        rarity_service: Arc<RarityService<Arc<R>>>,
        roulette: Roulette<S>,
        event_publisher: BroadcastEventPublisher,
        timeout: Duration,
    ) -> Self {
        Self {
            queue_repo,
            rarity_service,
            roulette,
            event_publisher,
            timeout,
        }
    }

    pub async fn dequeue_next(&self) -> Result<(QueueEntry, RouletteSlot), QueueServiceError> {
        let slot = self.roulette.roll().ok_or(QueueServiceError::NoSlots)?;

        let entry = match self.queue_repo.dequeue_next_with_slot(slot.id).await? {
            DequeueOutcome::Picked(entry) => entry,
            DequeueOutcome::AlreadyActive => return Err(QueueServiceError::AlreadyActive),
            DequeueOutcome::Empty => return Err(QueueServiceError::QueueEmpty),
        };

        let slot_rarity = self
            .rarity_service
            .get_by_id(slot.rarity_id)
            .map(|r| r.display_name)
            .ok_or(QueueServiceError::RarityNotFound)?;

        if let Err(e) = self
            .event_publisher
            .publish_spin(SpinEvent::Started {
                entry_id: entry.id,
                slot_name: slot.name.clone(),
                slot_rarity,
                user_name: entry.user_name.clone(),
            })
            .await
        {
            tracing::warn!("failed to publish spin_started event: {e}");
        }

        Ok((entry, slot))
    }

    pub async fn complete(&self, id: QueueEntryId) -> Result<(), QueueServiceError> {
        let updated = match self
            .queue_repo
            .update_status_if(id, QueueStatus::Spinning, QueueStatus::Completed)
            .await?
        {
            StatusUpdateOutcome::Updated(entry) => entry,
            StatusUpdateOutcome::NotFound => return Err(QueueServiceError::NotFound),
            StatusUpdateOutcome::StatusMismatch => return Err(QueueServiceError::NotSpinning),
        };

        if let Err(e) = self
            .event_publisher
            .publish_spin(SpinEvent::Completed {
                entry_id: updated.id,
            })
            .await
        {
            tracing::warn!("failed to publish spin_completed event: {e}");
        }

        Ok(())
    }

    pub async fn cancel(&self, id: QueueEntryId) -> Result<(), QueueServiceError> {
        let entry = self
            .queue_repo
            .get_by_id(id)
            .await?
            .ok_or(QueueServiceError::NotFound)?;

        if entry.status != QueueStatus::Pending && entry.status != QueueStatus::Error {
            return Err(QueueServiceError::NotCancellable);
        }

        match self
            .queue_repo
            .update_status_if(id, entry.status, QueueStatus::Cancelled)
            .await?
        {
            StatusUpdateOutcome::Updated(_) => {}
            StatusUpdateOutcome::NotFound => return Err(QueueServiceError::NotFound),
            StatusUpdateOutcome::StatusMismatch => return Err(QueueServiceError::NotCancellable),
        }

        Ok(())
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub async fn mark_timed_out(&self) -> Result<(), QueueServiceError> {
        let cutoff = Utc::now()
            .checked_sub_signed(chrono::Duration::seconds(self.timeout.as_secs() as i64))
            .unwrap_or_else(Utc::now);
        let entries = self.queue_repo.mark_timed_out(cutoff).await?;
        for entry in entries {
            if let Err(e) = self
                .event_publisher
                .publish_spin(SpinEvent::Error { entry_id: entry.id })
                .await
            {
                tracing::warn!(
                    "failed to publish spin_error event for entry {}: {e}",
                    entry.id
                );
            }
        }
        Ok(())
    }

    pub async fn enqueue(
        &self,
        user_id: UserId,
        user_name: &str,
    ) -> Result<QueueEntry, QueueServiceError> {
        Ok(self.queue_repo.enqueue(user_id, user_name).await?)
    }

    pub async fn peek_next(&self) -> Result<Option<QueueEntry>, QueueServiceError> {
        Ok(self.queue_repo.peek_next().await?)
    }

    pub async fn list(
        &self,
        status: Option<QueueStatus>,
    ) -> Result<Vec<QueueEntry>, QueueServiceError> {
        Ok(self.queue_repo.list(status).await?)
    }

    pub async fn get_by_id(
        &self,
        id: QueueEntryId,
    ) -> Result<Option<QueueEntry>, QueueServiceError> {
        Ok(self.queue_repo.get_by_id(id).await?)
    }

    pub async fn count_by_status(&self) -> Result<QueueStats, QueueServiceError> {
        Ok(self.queue_repo.count_by_status().await?)
    }
}

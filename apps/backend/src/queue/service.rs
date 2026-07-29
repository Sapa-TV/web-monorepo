use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::db::inmemory_queue::InMemoryQueueRepository;
use crate::db::inmemory_rarity::InMemoryRarityRepository;
use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
use crate::db::inmemory_user::InMemoryUserRepository;
use crate::error::QueueServiceError;
use crate::event::BroadcastEventPublisher;
use crate::queue::entry::{QueueEntry, QueueEntryId, QueueStatus};
use crate::queue::events::{SpinEvent, SpinEventPublisher};
use crate::queue::repository::QueueRepository;
use crate::random::StandartRandomProvider;
use crate::roulette::machine::RandomProvider;
use crate::roulette::rarity::RarityRepository;
use crate::roulette::repository::RouletteSlotRepository;
use crate::roulette::slot_service::RouletteSlot;
use crate::user::repository::UserRepository;

#[derive(Clone)]
#[non_exhaustive]
pub struct QueueService {
    queue_repo: Arc<InMemoryQueueRepository>,
    slot_repo: Arc<InMemoryRouletteSlotRepository>,
    rarity_repo: Arc<InMemoryRarityRepository>,
    user_repo: Arc<InMemoryUserRepository>,
    random: StandartRandomProvider,
    event_publisher: BroadcastEventPublisher,
    timeout: Duration,
}

impl QueueService {
    pub fn new(
        queue_repo: Arc<InMemoryQueueRepository>,
        slot_repo: Arc<InMemoryRouletteSlotRepository>,
        rarity_repo: Arc<InMemoryRarityRepository>,
        user_repo: Arc<InMemoryUserRepository>,
        random: StandartRandomProvider,
        event_publisher: BroadcastEventPublisher,
        timeout: Duration,
    ) -> Self {
        Self {
            queue_repo,
            slot_repo,
            rarity_repo,
            user_repo,
            random,
            event_publisher,
            timeout,
        }
    }

    fn pick_slot(&self, slots: &[RouletteSlot]) -> Option<RouletteSlot> {
        if slots.is_empty() {
            return None;
        }
        let total_weight: u64 = slots.iter().map(|s| s.weight).sum();
        if total_weight == 0 {
            return slots.last().cloned();
        }
        let threshold = (self.random.next() * total_weight as f64) as u64;
        let mut cumulative = 0u64;
        for slot in slots {
            cumulative += slot.weight;
            if threshold < cumulative {
                return Some(slot.clone());
            }
        }
        slots.last().cloned()
    }

    pub async fn dequeue_next(&self) -> Result<(QueueEntry, RouletteSlot), QueueServiceError> {
        let active = self
            .queue_repo
            .list(Some(QueueStatus::Spinning))
            .await?
            .into_iter()
            .next();

        if active.is_some() {
            return Err(QueueServiceError::AlreadyActive);
        }

        let entry = self
            .queue_repo
            .dequeue_next()
            .await?
            .ok_or(QueueServiceError::QueueEmpty)?;

        let slots = self.slot_repo.load_all().await?;
        let slot = self.pick_slot(&slots).ok_or(QueueServiceError::NoSlots)?;

        let entry = self
            .queue_repo
            .update_status(entry.id, QueueStatus::Spinning, Some(slot.id))
            .await?
            .ok_or(QueueServiceError::NotFound)?;

        let user = self
            .user_repo
            .get_by_id(entry.user_id)
            .await?
            .ok_or(QueueServiceError::UserNotFound)?;

        let rarities = self.rarity_repo.load_all().await?;
        let slot_rarity = rarities
            .iter()
            .find(|r| r.id == slot.rarity_id)
            .map(|r| r.display_name.clone())
            .ok_or(QueueServiceError::RarityNotFound)?;

        self.event_publisher
            .publish_spin(SpinEvent::Started {
                entry_id: entry.id,
                slot_name: slot.name.clone(),
                slot_rarity,
                user_name: user.display_name,
            })
            .await?;

        Ok((entry, slot))
    }

    pub async fn complete(&self, id: QueueEntryId) -> Result<(), QueueServiceError> {
        let entry = self
            .queue_repo
            .get_by_id(id)
            .await?
            .ok_or(QueueServiceError::NotFound)?;

        if entry.status != QueueStatus::Spinning {
            return Err(QueueServiceError::NotSpinning);
        }

        let updated = self
            .queue_repo
            .update_status(id, QueueStatus::Completed, entry.result_slot_id)
            .await?
            .ok_or(QueueServiceError::NotFound)?;

        self.event_publisher
            .publish_spin(SpinEvent::Completed {
                entry_id: updated.id,
            })
            .await?;

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

        self.queue_repo
            .update_status(id, QueueStatus::Cancelled, entry.result_slot_id)
            .await?
            .ok_or(QueueServiceError::NotFound)?;

        Ok(())
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub async fn mark_timed_out(&self) -> Result<(), QueueServiceError> {
        let cutoff = Utc::now()
            .naive_utc()
            .checked_sub_signed(chrono::Duration::seconds(self.timeout.as_secs() as i64))
            .unwrap_or(Utc::now().naive_utc());
        let entries = self.queue_repo.mark_timed_out(cutoff).await?;
        for entry in entries {
            let _ = self
                .event_publisher
                .publish_spin(SpinEvent::Error { entry_id: entry.id })
                .await;
        }
        Ok(())
    }
}

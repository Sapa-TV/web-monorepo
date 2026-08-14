use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::config::store::SharedSettings;
use crate::error::QueueServiceError;
use crate::event::BroadcastEventPublisher;
use crate::queue::entry::{QueueEntry, QueueEntryId, QueuePage, QueueStats, QueueStatus};
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

const MAX_QUEUE_PAGE_LIMIT: usize = 100;

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
    settings: SharedSettings,
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
            settings: self.settings.clone(),
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
        settings: SharedSettings,
    ) -> Self {
        Self {
            queue_repo,
            rarity_service,
            roulette,
            event_publisher,
            settings,
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
        Duration::from_secs(self.settings.read().roulette_timeout_secs)
    }

    pub async fn mark_timed_out(&self) -> Result<(), QueueServiceError> {
        let timeout = Duration::from_secs(self.settings.read().roulette_timeout_secs);
        let cutoff = Utc::now()
            .checked_sub_signed(chrono::Duration::seconds(timeout.as_secs() as i64))
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
        cursor: Option<QueueEntryId>,
        limit: usize,
    ) -> Result<QueuePage, QueueServiceError> {
        let limit = limit.clamp(1, MAX_QUEUE_PAGE_LIMIT);
        let mut entries = self.queue_repo.list(status, cursor, limit + 1).await?;
        let next_cursor = if entries.len() > limit {
            entries.truncate(limit);
            entries.last().map(|e| e.id)
        } else {
            None
        };
        Ok(QueuePage {
            entries,
            next_cursor,
        })
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

    pub async fn purge_expired(&self) -> Result<usize, QueueServiceError> {
        let retention = Duration::from_secs(self.settings.read().retention_secs);
        let cutoff = Utc::now()
            .checked_sub_signed(chrono::Duration::seconds(retention.as_secs() as i64))
            .unwrap_or(Utc::now());
        Ok(self.queue_repo.purge_completed_cancelled(cutoff).await?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;

    use crate::db::inmemory_config::InMemoryConfigRepository;
    use crate::db::inmemory_queue::InMemoryQueueRepository;
    use crate::error::QueueServiceError;
    use crate::queue::entry::QueueStatus;
    use crate::roulette::rarity::{Rarity, RarityId};
    use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
    use crate::state::AppState;
    use crate::test_fixtures::{test_state, test_state_with_data};
    use crate::user::UserId;

    use super::*;

    async fn setup_slots(state: &AppState) -> UserId {
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
        let state = test_state().await;
        let user_id = setup_slots(&state).await;
        state.queue_service.enqueue(user_id, "user1").await.unwrap();

        let (entry, slot) = state.queue_service.dequeue_next().await.unwrap();
        assert_eq!(entry.status, QueueStatus::Spinning);
        assert_eq!(slot.name, "test_slot");
        assert_eq!(entry.result_slot_id, Some(slot.id));
    }

    #[tokio::test]
    async fn dequeue_next_returns_409_when_already_active() {
        let state = test_state().await;
        let user_id = setup_slots(&state).await;
        state.queue_service.enqueue(user_id, "user1").await.unwrap();
        state.queue_service.dequeue_next().await.unwrap();

        let err = state.queue_service.dequeue_next().await.unwrap_err();
        assert!(matches!(err, QueueServiceError::AlreadyActive));
    }

    #[tokio::test]
    async fn dequeue_next_parallel_only_one_spin() {
        let state = test_state().await;
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
        let queue_repo = Arc::new(InMemoryQueueRepository::new());
        let config_repo = Arc::new(InMemoryConfigRepository::new());
        let state = test_state_with_data(Arc::clone(&queue_repo), config_repo).await;
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
        let state = test_state().await;
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
        let state = test_state().await;
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
        let state = test_state().await;
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
        let state = test_state().await;
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
        let state = test_state().await;

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
}

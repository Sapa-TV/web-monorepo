use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::nonpoison::Mutex;

use chrono::Utc;

use model::error::RepositoryError;
use model::push::repository::PushSubscriptionRepository;
use model::push::{NewPushSubscription, PushSubscription, PushSubscriptionId};
use model::user::UserId;

#[non_exhaustive]
pub struct InMemoryPushSubscriptionRepository {
    subscriptions: Mutex<Vec<PushSubscription>>,
    next_id: AtomicU32,
}

impl InMemoryPushSubscriptionRepository {
    pub fn new() -> Self {
        Self {
            subscriptions: Mutex::new(Vec::new()),
            next_id: AtomicU32::new(1),
        }
    }
}

impl Default for InMemoryPushSubscriptionRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl PushSubscriptionRepository for InMemoryPushSubscriptionRepository {
    async fn upsert(
        &self,
        subscription: &NewPushSubscription,
    ) -> Result<PushSubscription, RepositoryError> {
        let now = Utc::now();
        let mut subscriptions = self.subscriptions.lock();
        if let Some(existing) = subscriptions
            .iter_mut()
            .find(|it| it.endpoint == subscription.endpoint)
        {
            existing.user_id = subscription.user_id;
            existing.p256dh = subscription.p256dh.clone();
            existing.auth = subscription.auth.clone();
            existing.user_agent = subscription.user_agent.clone();
            existing.last_seen_at = now;
            existing.failure_count = 0;
            return Ok(existing.clone());
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let created = PushSubscription::new(
            PushSubscriptionId::new(id),
            subscription.user_id,
            subscription.endpoint.clone(),
            subscription.p256dh.clone(),
            subscription.auth.clone(),
            subscription.user_agent.clone(),
            now,
            now,
            0,
        );
        subscriptions.push(created.clone());
        Ok(created)
    }

    async fn list_all(&self) -> Result<Vec<PushSubscription>, RepositoryError> {
        Ok(self.subscriptions.lock().clone())
    }

    async fn list_by_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<PushSubscription>, RepositoryError> {
        Ok(self
            .subscriptions
            .lock()
            .iter()
            .filter(|it| it.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn delete_by_user_and_endpoint(
        &self,
        user_id: UserId,
        endpoint: &str,
    ) -> Result<bool, RepositoryError> {
        let mut subscriptions = self.subscriptions.lock();
        let len_before = subscriptions.len();
        subscriptions.retain(|it| !(it.user_id == user_id && it.endpoint == endpoint));
        Ok(subscriptions.len() != len_before)
    }

    async fn record_delivery(&self, id: PushSubscriptionId) -> Result<(), RepositoryError> {
        let mut subscriptions = self.subscriptions.lock();
        if let Some(subscription) = subscriptions.iter_mut().find(|it| it.id == id) {
            subscription.last_seen_at = Utc::now();
            subscription.failure_count = 0;
        }
        Ok(())
    }

    async fn record_failure(&self, id: PushSubscriptionId) -> Result<(), RepositoryError> {
        let mut subscriptions = self.subscriptions.lock();
        if let Some(subscription) = subscriptions.iter_mut().find(|it| it.id == id) {
            subscription.failure_count += 1;
        }
        Ok(())
    }

    async fn delete(&self, id: PushSubscriptionId) -> Result<bool, RepositoryError> {
        let mut subscriptions = self.subscriptions.lock();
        let len_before = subscriptions.len();
        subscriptions.retain(|it| it.id != id);
        Ok(subscriptions.len() != len_before)
    }
}

#[cfg(test)]
#[path = "inmemory_push.test.rs"]
mod tests;

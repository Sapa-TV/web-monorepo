use std::sync::Arc;

use model::push::repository::PushSubscriptionRepository;
use model::push::{NewPushSubscription, PushSubscription};
use model::user::UserId;

use crate::error::PushServiceError;

#[derive(Debug, Clone, serde::Serialize)]
#[non_exhaustive]
pub struct PushPayload {
    pub title: String,
    pub body: String,
    pub url: String,
    pub tag: String,
}

impl PushPayload {
    pub fn new(
        title: impl Into<String>,
        body: impl Into<String>,
        url: impl Into<String>,
        tag: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            url: url.into(),
            tag: tag.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PushOutcome {
    Delivered,
    Gone,
    Retry,
}

pub trait PushSender: Send + Sync {
    fn send(
        &self,
        subscription: &PushSubscription,
        payload: &PushPayload,
    ) -> impl Future<Output = Result<PushOutcome, String>> + Send;
}

pub struct PushService<R, S>
where
    R: PushSubscriptionRepository,
    S: PushSender,
{
    repo: Arc<R>,
    sender: Arc<S>,
}

impl<R, S> PushService<R, S>
where
    R: PushSubscriptionRepository,
    S: PushSender,
{
    pub fn new(repo: Arc<R>, sender: Arc<S>) -> Self {
        Self { repo, sender }
    }

    pub async fn subscribe(
        &self,
        user_id: UserId,
        endpoint: &str,
        p256dh: &str,
        auth: &str,
        user_agent: Option<&str>,
    ) -> Result<PushSubscription, PushServiceError> {
        if !endpoint.starts_with("https://") {
            return Err(PushServiceError::InvalidSubscription(
                "endpoint must use https".to_string(),
            ));
        }
        if p256dh.is_empty() || auth.is_empty() {
            return Err(PushServiceError::InvalidSubscription(
                "missing keys".to_string(),
            ));
        }
        let subscription = NewPushSubscription::new(
            user_id,
            endpoint,
            p256dh,
            auth,
            user_agent.map(str::to_string),
        );
        Ok(self.repo.upsert(&subscription).await?)
    }

    pub async fn unsubscribe(
        &self,
        user_id: UserId,
        endpoint: &str,
    ) -> Result<bool, PushServiceError> {
        Ok(self
            .repo
            .delete_by_user_and_endpoint(user_id, endpoint)
            .await?)
    }

    pub async fn subscriptions(
        &self,
        user_id: UserId,
    ) -> Result<Vec<PushSubscription>, PushServiceError> {
        Ok(self.repo.list_by_user(user_id).await?)
    }

    pub async fn broadcast(&self, payload: &PushPayload) -> Result<usize, PushServiceError> {
        let subscriptions = self.repo.list_all().await?;
        let mut delivered = 0;
        for subscription in subscriptions {
            match self.sender.send(&subscription, payload).await {
                Ok(PushOutcome::Delivered) => {
                    delivered += 1;
                    self.repo.record_delivery(subscription.id).await.ok();
                }
                Ok(PushOutcome::Gone) => {
                    self.repo.delete(subscription.id).await.ok();
                }
                Ok(PushOutcome::Retry) => {
                    self.repo.record_failure(subscription.id).await.ok();
                }
                Err(error) => {
                    tracing::warn!(
                        subscription_id = subscription.id.get(),
                        "push send failed: {error}"
                    );
                    self.repo.record_failure(subscription.id).await.ok();
                }
            }
        }
        Ok(delivered)
    }
}

#[cfg(test)]
#[path = "service.test.rs"]
mod tests;

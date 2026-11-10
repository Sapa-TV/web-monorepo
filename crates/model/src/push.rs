pub mod repository;

use chrono::{DateTime, Utc};

use crate::user::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PushSubscriptionId(u32);

impl PushSubscriptionId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn get(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NewPushSubscription {
    pub user_id: UserId,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub user_agent: Option<String>,
}

impl NewPushSubscription {
    pub fn new(
        user_id: UserId,
        endpoint: impl Into<String>,
        p256dh: impl Into<String>,
        auth: impl Into<String>,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            user_id,
            endpoint: endpoint.into(),
            p256dh: p256dh.into(),
            auth: auth.into(),
            user_agent,
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PushSubscription {
    pub id: PushSubscriptionId,
    pub user_id: UserId,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub failure_count: u32,
}

impl PushSubscription {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: PushSubscriptionId,
        user_id: UserId,
        endpoint: String,
        p256dh: String,
        auth: String,
        user_agent: Option<String>,
        created_at: DateTime<Utc>,
        last_seen_at: DateTime<Utc>,
        failure_count: u32,
    ) -> Self {
        Self {
            id,
            user_id,
            endpoint,
            p256dh,
            auth,
            user_agent,
            created_at,
            last_seen_at,
            failure_count,
        }
    }
}

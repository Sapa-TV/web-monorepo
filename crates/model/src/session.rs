pub mod repository;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::platform::PlatformId;
use crate::user::UserId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[non_exhaustive]
pub struct SessionToken(String);

impl SessionToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<SessionToken> for String {
    fn from(token: SessionToken) -> Self {
        token.0
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Session {
    pub token: SessionToken,
    pub user_id: UserId,
    pub platform_id: Option<PlatformId>,
    pub platform_user_id: Option<String>,
    pub platform_username: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    _sealed: (),
}

impl Session {
    pub fn new(
        token: SessionToken,
        user_id: UserId,
        platform_id: Option<PlatformId>,
        platform_user_id: Option<String>,
        platform_username: Option<String>,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            token,
            user_id,
            platform_id,
            platform_user_id,
            platform_username,
            created_at,
            expires_at,
            _sealed: (),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LoginTicketToken(String);

impl LoginTicketToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LoginTicket {
    pub ticket: LoginTicketToken,
    pub platform_id: PlatformId,
    pub platform_user_id: String,
    pub platform_username: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    _sealed: (),
}

impl LoginTicket {
    pub fn new(
        ticket: LoginTicketToken,
        platform_id: PlatformId,
        platform_user_id: String,
        platform_username: Option<String>,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            ticket,
            platform_id,
            platform_user_id,
            platform_username,
            created_at,
            expires_at,
            _sealed: (),
        }
    }
}

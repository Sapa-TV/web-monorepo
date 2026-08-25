pub mod repository;
pub mod service;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[non_exhaustive]
pub struct SessionToken(String);

impl SessionToken {
    pub(crate) fn new(token: impl Into<String>) -> Self {
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
    pub twitch_user_id: String,
    pub twitch_user_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    _sealed: (),
}

impl Session {
    pub fn new(
        token: SessionToken,
        twitch_user_id: String,
        twitch_user_name: Option<String>,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            token,
            twitch_user_id,
            twitch_user_name,
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
    pub(crate) fn new(token: impl Into<String>) -> Self {
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
    pub twitch_user_id: String,
    pub twitch_user_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    _sealed: (),
}

impl LoginTicket {
    pub fn new(
        ticket: LoginTicketToken,
        twitch_user_id: String,
        twitch_user_name: Option<String>,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            ticket,
            twitch_user_id,
            twitch_user_name,
            created_at,
            expires_at,
            _sealed: (),
        }
    }
}

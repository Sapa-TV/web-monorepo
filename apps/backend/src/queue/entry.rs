use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use utoipa::ToSchema;

use crate::roulette::slot_service::RouletteSlotId;
use crate::user::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct QueueEntryId(u32);

impl QueueEntryId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }
}

impl Display for QueueEntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[non_exhaustive]
pub enum QueueStatus {
    Pending,
    Spinning,
    Completed,
    Error,
    Cancelled,
}

impl<'de> Deserialize<'de> for QueueStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.to_ascii_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "spinning" => Ok(Self::Spinning),
            "completed" => Ok(Self::Completed),
            "error" => Ok(Self::Error),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(serde::de::Error::custom(format!(
                "unknown queue status: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct QueueEntry {
    pub id: QueueEntryId,
    pub user_id: UserId,
    pub user_name: String,
    pub status: QueueStatus,
    pub result_slot_id: Option<RouletteSlotId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl QueueEntry {
    pub fn new(
        id: QueueEntryId,
        user_id: UserId,
        user_name: impl Into<String>,
        status: QueueStatus,
        result_slot_id: Option<RouletteSlotId>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            user_name: user_name.into(),
            status,
            result_slot_id,
            created_at,
            updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[non_exhaustive]
pub struct QueueStats {
    pub pending: u32,
    pub spinning: u32,
    pub completed: u32,
    pub error: u32,
    pub cancelled: u32,
}

impl QueueStats {
    pub fn new(pending: u32, spinning: u32, completed: u32, error: u32, cancelled: u32) -> Self {
        Self {
            pending,
            spinning,
            completed,
            error,
            cancelled,
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct QueuePage {
    pub entries: Vec<QueueEntry>,
    pub next_cursor: Option<QueueEntryId>,
}

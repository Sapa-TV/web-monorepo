use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::roulette::slot_service::RouletteSlotId;
use crate::user::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct QueueEntryId(u32);

impl QueueEntryId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }

    pub(crate) const fn value(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub enum QueueStatus {
    Pending,
    Spinning,
    Completed,
    Error,
    Cancelled,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct QueueEntry {
    pub id: QueueEntryId,
    pub user_id: UserId,
    pub user_name: String,
    pub status: QueueStatus,
    pub result_slot_id: Option<RouletteSlotId>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl QueueEntry {
    pub fn new(
        id: QueueEntryId,
        user_id: UserId,
        user_name: impl Into<String>,
        status: QueueStatus,
        result_slot_id: Option<RouletteSlotId>,
        created_at: NaiveDateTime,
        updated_at: NaiveDateTime,
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

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::roulette::slot_service::RouletteSlotId;
use crate::user::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
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
pub enum QueueStatus {
    Pending,
    Spinning,
    Completed,
    Error,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub id: QueueEntryId,
    pub user_id: UserId,
    pub status: QueueStatus,
    pub result_slot_id: Option<RouletteSlotId>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QueueStats {
    pub pending: u32,
    pub spinning: u32,
    pub completed: u32,
    pub error: u32,
    pub cancelled: u32,
}

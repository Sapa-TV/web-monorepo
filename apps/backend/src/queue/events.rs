use serde::Serialize;

use crate::error::event::EventError;
use crate::queue::entry::QueueEntryId;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum SpinEvent {
    #[serde(rename = "spin_started")]
    Started {
        entry_id: QueueEntryId,
        slot_name: String,
        slot_rarity: String,
        user_name: String,
    },
    #[serde(rename = "spin_completed")]
    Completed {
        entry_id: QueueEntryId,
    },
    #[serde(rename = "spin_error")]
    Error {
        entry_id: QueueEntryId,
    },
}

pub trait SpinEventPublisher: Send + Sync {
    async fn publish_spin(&self, event: SpinEvent) -> Result<(), EventError>;
}

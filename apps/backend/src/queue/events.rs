use crate::error::event::EventError;
use crate::queue::entry::QueueEntryId;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SpinEvent {
    Started {
        entry_id: QueueEntryId,
        slot_name: String,
        slot_rarity: String,
        user_name: String,
    },
    Completed {
        entry_id: QueueEntryId,
    },
    Error {
        entry_id: QueueEntryId,
    },
}

pub trait SpinEventPublisher: Send + Sync {
    async fn publish_spin(&self, event: SpinEvent) -> Result<(), EventError>;
}

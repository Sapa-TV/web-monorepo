use std::sync::Arc;

use tokio::sync::broadcast;

use crate::error::event::EventError;
use crate::queue::events::{SpinEvent, SpinEventPublisher};

#[derive(Clone)]
#[non_exhaustive]
pub struct BroadcastEventPublisher {
    tx: broadcast::Sender<Arc<SpinEvent>>,
}

impl BroadcastEventPublisher {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(16);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<SpinEvent>> {
        self.tx.subscribe()
    }
}

impl Default for BroadcastEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl SpinEventPublisher for BroadcastEventPublisher {
    async fn publish_spin(&self, event: SpinEvent) -> Result<(), EventError> {
        match self.tx.send(Arc::new(event)) {
            Ok(_) => Ok(()),
            Err(_) => {
                tracing::warn!("spin event dropped: no subscribers");
                Err(EventError::Publish("no subscribers".to_string()))
            }
        }
    }
}

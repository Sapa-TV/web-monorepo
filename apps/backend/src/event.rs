use crate::error::event::EventError;
use crate::queue::events::{SpinEvent, SpinEventPublisher};

#[derive(Clone)]
pub struct NoopEventPublisher;

impl SpinEventPublisher for NoopEventPublisher {
    async fn publish_spin(&self, _event: SpinEvent) -> Result<(), EventError> {
        Ok(())
    }
}

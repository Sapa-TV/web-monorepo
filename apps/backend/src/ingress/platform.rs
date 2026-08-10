use std::future::Future;

use tokio::sync::mpsc;

use crate::error::ingress::PlatformError;
use crate::ingress::event::PlatformEvent;
use crate::platform::Platform;

pub type EventSink = mpsc::Sender<PlatformEvent>;

pub trait PlatformService: Send + Sync {
    fn platform(&self) -> Platform;

    fn run(&self, sink: EventSink) -> impl Future<Output = Result<(), PlatformError>> + Send;
}

use std::future::Future;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::ingress::PlatformError;
use crate::ingress::event::PlatformEvent;
use crate::platform::{Platform, PlatformId};

pub type EventSink = mpsc::Sender<PlatformEvent>;

pub trait PlatformService: Send + Sync {
    fn platform(&self) -> Platform;

    fn run(
        &self,
        sink: EventSink,
        shutdown: CancellationToken,
    ) -> impl Future<Output = Result<(), PlatformError>> + Send;
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ConnectedIdentity {
    pub user_id: String,
    pub user_name: String,
}

pub trait PlatformAuth: Send + Sync {
    fn platform(&self) -> PlatformId;

    fn is_configured(&self) -> impl Future<Output = Result<bool, PlatformError>> + Send;

    fn connect(
        &self,
        code: &str,
    ) -> impl Future<Output = Result<ConnectedIdentity, PlatformError>> + Send;

    fn revoke(&self) -> impl Future<Output = Result<(), PlatformError>> + Send;
}

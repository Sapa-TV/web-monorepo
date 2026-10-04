use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::ingress::platform::EventSink;
use crate::platform::{PlatformCredentialRepository, PlatformCredentialService, PlatformId};

const INGRESS_STOP_GRACE: Duration = Duration::from_secs(5);

struct RunningIngress {
    join: JoinHandle<()>,
    token: CancellationToken,
}

async fn stop_ingress(platform: PlatformId, mut ingress: RunningIngress, grace: Duration) {
    ingress.token.cancel();
    if timeout(grace, &mut ingress.join).await.is_err() {
        tracing::warn!(?platform, "ingress graceful stop timed out; aborting");
        ingress.join.abort();
    }
}

#[non_exhaustive]
pub struct IngressSupervisor<C>
where
    C: PlatformCredentialRepository,
{
    credentials: Arc<PlatformCredentialService<C>>,
    sink: EventSink,
    platforms: &'static [PlatformId],
}

impl<C> IngressSupervisor<C>
where
    C: PlatformCredentialRepository,
{
    pub fn new(
        credentials: Arc<PlatformCredentialService<C>>,
        sink: EventSink,
        platforms: &'static [PlatformId],
    ) -> Self {
        Self {
            credentials,
            sink,
            platforms,
        }
    }

    /// Reconciles once at startup, then on every lifecycle signal.
    /// Cancelling `shutdown` stops the supervisor and every managed ingress.
    pub async fn run<F>(self, spawn: F, shutdown: CancellationToken)
    where
        F: Fn(
                PlatformId,
                Arc<PlatformCredentialService<C>>,
                EventSink,
                CancellationToken,
            ) -> Option<JoinHandle<()>>
            + Send
            + Sync
            + 'static,
    {
        let mut running: HashMap<PlatformId, RunningIngress> = HashMap::new();
        let mut lifecycle = self.credentials.subscribe_lifecycle();

        self.reconcile(&spawn, &shutdown, &mut running).await;

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                changed = lifecycle.changed() => {
                    if changed.is_err() {
                        break;
                    }
                }
            }
            self.reconcile(&spawn, &shutdown, &mut running).await;
        }

        for (platform, ingress) in running {
            stop_ingress(platform, ingress, INGRESS_STOP_GRACE).await;
        }
    }

    async fn reconcile<F>(
        &self,
        spawn: &F,
        shutdown: &CancellationToken,
        running: &mut HashMap<PlatformId, RunningIngress>,
    ) where
        F: Fn(
                PlatformId,
                Arc<PlatformCredentialService<C>>,
                EventSink,
                CancellationToken,
            ) -> Option<JoinHandle<()>>
            + Send
            + Sync,
    {
        for &platform in self.platforms {
            let configured = self
                .credentials
                .load_credential(platform)
                .await
                .ok()
                .flatten()
                .is_some();
            let is_running = running.contains_key(&platform);
            match (configured, is_running) {
                (false, false) => {}
                (false, true) => {
                    let ingress = running.remove(&platform).expect("running platform");
                    stop_ingress(platform, ingress, INGRESS_STOP_GRACE).await;
                    tracing::info!(?platform, "ingress supervisor: stopped");
                }
                (true, false) => {
                    let token = shutdown.child_token();
                    if let Some(join) = spawn(
                        platform,
                        Arc::clone(&self.credentials),
                        self.sink.clone(),
                        token.clone(),
                    ) {
                        running.insert(platform, RunningIngress { join, token });
                        tracing::info!(?platform, "ingress supervisor: started");
                    } else {
                        tracing::warn!(
                            ?platform,
                            "ingress supervisor: no factory for configured platform"
                        );
                    }
                }
                (true, true) => {
                    let ingress = running.remove(&platform).expect("running platform");
                    stop_ingress(platform, ingress, INGRESS_STOP_GRACE).await;
                    let token = shutdown.child_token();
                    if let Some(join) = spawn(
                        platform,
                        Arc::clone(&self.credentials),
                        self.sink.clone(),
                        token.clone(),
                    ) {
                        running.insert(platform, RunningIngress { join, token });
                        tracing::info!(?platform, "ingress supervisor: restarted");
                    } else {
                        tracing::warn!(
                            ?platform,
                            "ingress supervisor: factory no longer returns an ingress"
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "supervisor.test.rs"]
mod tests;

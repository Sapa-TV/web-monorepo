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
mod tests {
    use std::collections::HashMap;
    use std::future::pending;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use tokio::sync::mpsc;
    use tokio::task::{AbortHandle, JoinHandle};
    use tokio::time::{sleep, timeout};
    use tokio_util::sync::CancellationToken;

    use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
    use crate::ingress::event::PlatformEvent;
    use crate::ingress::platform::EventSink;
    use crate::ingress::supervisor::IngressSupervisor;
    use crate::platform::{PlatformCredentialRepository, PlatformCredentialService, PlatformId};

    type TestRepo = InMemoryPlatformCredentialRepository;

    fn test_service() -> Arc<PlatformCredentialService<TestRepo>> {
        Arc::new(PlatformCredentialService::new(Arc::new(
            InMemoryPlatformCredentialRepository::new(),
        )))
    }

    fn test_sink() -> EventSink {
        let (sink, _rx) = mpsc::channel::<PlatformEvent>(16);
        sink
    }

    struct Stub {
        spawns: Arc<Mutex<Vec<PlatformId>>>,
        handles: Arc<Mutex<Vec<AbortHandle>>>,
    }

    impl Stub {
        fn new() -> Self {
            Self {
                spawns: Arc::new(Mutex::new(Vec::new())),
                handles: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn factory<C>(
            &self,
        ) -> impl Fn(
            PlatformId,
            Arc<PlatformCredentialService<C>>,
            EventSink,
            CancellationToken,
        ) -> Option<JoinHandle<()>>
        + Send
        + Sync
        + 'static
        where
            C: PlatformCredentialRepository,
        {
            let spawns = Arc::clone(&self.spawns);
            let handles = Arc::clone(&self.handles);
            move |platform, _credentials, _sink, token| {
                spawns.lock().unwrap().push(platform);
                let handle = tokio::spawn(async move {
                    token.cancelled().await;
                });
                let abort = handle.abort_handle();
                handles.lock().unwrap().push(abort);
                Some(handle)
            }
        }
    }

    async fn wait_for_finished(expected_finished: usize, handles: &Mutex<Vec<AbortHandle>>) {
        for _ in 0..100 {
            let finished = handles
                .lock()
                .unwrap()
                .iter()
                .filter(|h| h.is_finished())
                .count();
            if finished == expected_finished {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
        let finished = handles
            .lock()
            .unwrap()
            .iter()
            .filter(|h| h.is_finished())
            .count();
        assert_eq!(
            finished, expected_finished,
            "expected {expected_finished} aborted tasks"
        );
    }

    async fn wait_for_spawns(expected: usize, spawns: &Mutex<Vec<PlatformId>>) {
        for _ in 0..100 {
            if spawns.lock().unwrap().len() == expected {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            spawns.lock().unwrap().len(),
            expected,
            "unexpected number of spawns"
        );
    }

    #[tokio::test]
    async fn reconcile_starts_on_new_credential() {
        let credentials = test_service();
        let supervisor =
            IngressSupervisor::new(credentials.clone(), test_sink(), &[PlatformId::TWITCH]);
        let stub = Stub::new();
        let factory = stub.factory();
        let mut running = HashMap::new();

        supervisor
            .reconcile(&factory, &CancellationToken::new(), &mut running)
            .await;
        assert!(running.is_empty());

        credentials
            .save_credential(PlatformId::TWITCH, "tok-1")
            .await
            .unwrap();
        supervisor
            .reconcile(&factory, &CancellationToken::new(), &mut running)
            .await;

        assert_eq!(
            stub.spawns.lock().unwrap().as_slice(),
            &[PlatformId::TWITCH]
        );
        assert!(running.contains_key(&PlatformId::TWITCH));
    }

    #[tokio::test]
    async fn reconcile_restarts_on_replaced_credential() {
        let credentials = test_service();
        let supervisor =
            IngressSupervisor::new(credentials.clone(), test_sink(), &[PlatformId::TWITCH]);
        let stub = Stub::new();
        let factory = stub.factory();
        let mut running = HashMap::new();

        credentials
            .save_credential(PlatformId::TWITCH, "tok-1")
            .await
            .unwrap();
        supervisor
            .reconcile(&factory, &CancellationToken::new(), &mut running)
            .await;
        assert_eq!(stub.handles.lock().unwrap().len(), 1);

        credentials
            .save_credential(PlatformId::TWITCH, "tok-2")
            .await
            .unwrap();
        supervisor
            .reconcile(&factory, &CancellationToken::new(), &mut running)
            .await;

        assert_eq!(
            stub.spawns.lock().unwrap().len(),
            2,
            "replacing credentials must spawn a fresh ingress"
        );
        assert!(running.contains_key(&PlatformId::TWITCH));
        wait_for_finished(1, &stub.handles).await;
    }

    #[tokio::test]
    async fn reconcile_stops_on_revoked_credential() {
        let credentials = test_service();
        let supervisor =
            IngressSupervisor::new(credentials.clone(), test_sink(), &[PlatformId::TWITCH]);
        let stub = Stub::new();
        let factory = stub.factory();
        let mut running = HashMap::new();

        credentials
            .save_credential(PlatformId::TWITCH, "tok-1")
            .await
            .unwrap();
        supervisor
            .reconcile(&factory, &CancellationToken::new(), &mut running)
            .await;
        assert_eq!(stub.handles.lock().unwrap().len(), 1);

        credentials
            .clear_credential(PlatformId::TWITCH)
            .await
            .unwrap();
        supervisor
            .reconcile(&factory, &CancellationToken::new(), &mut running)
            .await;

        assert!(
            running.is_empty(),
            "revoked credentials must stop the ingress"
        );
        assert_eq!(
            stub.spawns.lock().unwrap().len(),
            1,
            "stopping must not spawn again"
        );
        wait_for_finished(1, &stub.handles).await;
    }

    #[tokio::test]
    async fn reconcile_ignores_unconfigured_platform() {
        let credentials = test_service();
        let supervisor =
            IngressSupervisor::new(credentials.clone(), test_sink(), &[PlatformId::YOUTUBE]);
        let stub = Stub::new();
        let factory = stub.factory();
        let mut running = HashMap::new();

        let supervisor2 = IngressSupervisor::new(
            credentials.clone(),
            test_sink(),
            &[PlatformId::VK_VIDEO_LIVE],
        );
        let stub2 = Stub::new();
        let factory2 = stub2.factory();
        let mut running2 = HashMap::new();

        credentials
            .save_credential(PlatformId::TWITCH, "tok-1")
            .await
            .unwrap();
        supervisor
            .reconcile(&factory, &CancellationToken::new(), &mut running)
            .await;
        supervisor2
            .reconcile(&factory2, &CancellationToken::new(), &mut running2)
            .await;

        assert!(
            running.is_empty(),
            "youtube without credentials must not start"
        );
        assert!(
            running2.is_empty(),
            "vkv without credentials must not start"
        );
        assert!(stub.spawns.lock().unwrap().is_empty());
        assert!(stub2.spawns.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn run_loop_drives_start_stop_rotation() {
        let credentials = test_service();
        let supervisor =
            IngressSupervisor::new(credentials.clone(), test_sink(), &[PlatformId::TWITCH]);
        let stub = Stub::new();
        let spawns = Arc::clone(&stub.spawns);
        let handles = Arc::clone(&stub.handles);

        let shutdown = CancellationToken::new();
        let task = tokio::spawn(supervisor.run(stub.factory(), shutdown));

        credentials
            .save_credential(PlatformId::TWITCH, "tok-1")
            .await
            .unwrap();
        wait_for_spawns(1, &spawns).await;

        credentials
            .save_rotated(PlatformId::TWITCH, "tok-rotated")
            .await
            .unwrap();
        sleep(Duration::from_millis(50)).await;
        assert_eq!(
            spawns.lock().unwrap().len(),
            1,
            "rotation must not restart the ingress"
        );
        assert_eq!(handles.lock().unwrap().len(), 1, "no restart on rotation");

        credentials
            .clear_credential(PlatformId::TWITCH)
            .await
            .unwrap();
        wait_for_finished(1, &handles).await;
        assert_eq!(spawns.lock().unwrap().len(), 1, "stop must not spawn");

        task.abort();
    }

    #[tokio::test]
    async fn run_loop_startup_reconciles_existing_credentials() {
        let credentials = Arc::new(PlatformCredentialService::new(Arc::new(
            InMemoryPlatformCredentialRepository::seeded([(PlatformId::TWITCH, "tok-seeded")]),
        )));
        let supervisor =
            IngressSupervisor::new(credentials.clone(), test_sink(), &[PlatformId::TWITCH]);
        let stub = Stub::new();
        let spawns = Arc::clone(&stub.spawns);
        let handles = Arc::clone(&stub.handles);

        let shutdown = CancellationToken::new();
        let task = tokio::spawn(supervisor.run(stub.factory(), shutdown.clone()));

        wait_for_spawns(1, &spawns).await;

        credentials
            .clear_credential(PlatformId::TWITCH)
            .await
            .unwrap();
        wait_for_finished(1, &handles).await;

        shutdown.cancel();
        timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn stop_ingress_aborts_task_that_ignores_cancellation() {
        let join = tokio::spawn(async {
            pending::<()>().await;
        });
        let ingress = super::RunningIngress {
            join,
            token: CancellationToken::new(),
        };

        let start = Instant::now();
        super::stop_ingress(PlatformId::TWITCH, ingress, Duration::from_millis(50)).await;

        assert!(
            start.elapsed() >= Duration::from_millis(50),
            "stop must wait out the grace period before aborting"
        );
    }
}

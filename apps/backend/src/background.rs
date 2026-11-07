use std::sync::Arc;
use std::time::Duration;

use tokio::select;
use tokio::signal::ctrl_c;
use tokio::time;
use tokio_util::sync::CancellationToken;
use tracing::info;

#[cfg(not(unix))]
use std::future::pending;
#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal};

use composition::runtime;
use composition::srs::stream_status_task;
use composition::state::{
    AppConfigStore, AppQueueService, AppSessionService, AppSheetsService, AppState,
};
use domain::sheets;

pub fn start_background_tasks(state: &AppState, shutdown: &CancellationToken) {
    tokio::spawn(queue_timeout_task(
        Arc::clone(&state.queue_service),
        shutdown.child_token(),
    ));
    tokio::spawn(queue_purge_task(
        Arc::clone(&state.queue_service),
        Arc::clone(&state.config),
        shutdown.child_token(),
    ));
    tokio::spawn(session_prune_task(
        Arc::clone(&state.session_service),
        Arc::clone(&state.config),
        shutdown.child_token(),
    ));
    tokio::spawn(sheets_sync_task(
        Arc::clone(&state.sheets),
        shutdown.child_token(),
    ));
    if let Some(api_url) = state.config.srs_api_url() {
        tokio::spawn(stream_status_task(
            Arc::clone(&state.stream_status),
            api_url.to_string(),
            shutdown.child_token(),
        ));
    }

    runtime::start_rule_pipeline(state, shutdown);
}

pub async fn signal_listener(token: CancellationToken) {
    let ctrl_c = async {
        ctrl_c().await.expect("failed to install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("shutdown signal received, cancelling root token");
    token.cancel();
}

async fn queue_timeout_task(queue_service: Arc<AppQueueService>, shutdown: CancellationToken) {
    loop {
        select! {
            biased;
            _ = shutdown.cancelled() => break,
            _ = time::sleep(queue_service.timeout()) => {},
        }
        if let Err(e) = queue_service.mark_timed_out().await {
            tracing::error!("mark_timed_out failed: {e}");
        }
    }
}

async fn queue_purge_task(
    queue_service: Arc<AppQueueService>,
    config: Arc<AppConfigStore>,
    shutdown: CancellationToken,
) {
    loop {
        let interval = Duration::from_secs(config.queue_cleanup_interval_secs());
        select! {
            biased;
            _ = shutdown.cancelled() => break,
            _ = time::sleep(interval) => {},
        }
        if let Err(e) = queue_service.purge_expired().await {
            tracing::error!("queue purge_expired failed: {e}");
        }
    }
}

async fn sheets_sync_task(sheets: Arc<AppSheetsService>, shutdown: CancellationToken) {
    let interval = Duration::from_secs(sheets::service::SYNC_CHECK_INTERVAL_SECS);
    loop {
        select! {
            biased;
            _ = shutdown.cancelled() => break,
            _ = time::sleep(interval) => {},
        }
        if let Err(e) = sheets.sync_if_due().await {
            tracing::error!("sheets sync failed: {e}");
        }
    }
}

async fn session_prune_task(
    session_service: Arc<AppSessionService>,
    config: Arc<AppConfigStore>,
    shutdown: CancellationToken,
) {
    loop {
        let interval = Duration::from_secs(config.sessions_cleanup_interval_secs());
        select! {
            biased;
            _ = shutdown.cancelled() => break,
            _ = time::sleep(interval) => {},
        }
        if let Err(e) = session_service.prune_expired().await {
            tracing::error!("session prune_expired failed: {e}");
        }
    }
}

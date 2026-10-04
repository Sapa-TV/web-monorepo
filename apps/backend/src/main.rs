#![deny(clippy::exhaustive_structs)]
#![deny(clippy::new_ret_no_self)]

use std::future::pending;
use std::sync::Arc;
use std::time::Duration;

use axum::http::{HeaderValue, Method, header};
use backend::api;
use backend::config::store::ConfigStore;
use backend::db::sqlite::config::SqliteConfigRepository;
use backend::db::sqlite::connect_from_env;
use backend::db::sqlite::platform_credential::SqlitePlatformCredentialRepository;
use backend::ingress::PlatformService;
use backend::ingress::platform::EventSink;
use backend::ingress::supervisor::IngressSupervisor;
use backend::ingress::twitch::TwitchPlatformService;
use backend::ingress::vk_video_live::VkVideoLivePlatformService;
use backend::ingress::vk_video_live_auth::VkVideoLiveAuthService;
use backend::openapi;
use backend::platform::{PlatformCredentialService, PlatformId};
use backend::random::StandartRandomProvider;
use backend::runtime;
use backend::sheets;
use backend::state::{
    AppConfigStore, AppQueueService, AppSessionService, AppSheetsService, AppState, AppStateBuilder,
};
use backend::widget_api;
use tokio::net::TcpListener;
use tokio::select;
use tokio::signal::ctrl_c;
use tokio::task::JoinHandle;
use tokio::time;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or(tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    dotenvy::dotenv().ok();
    let pool = connect_from_env().await.expect("failed to open database");
    let credentials_repo = Arc::new(SqlitePlatformCredentialRepository::new(pool.clone()));
    let config_store =
        ConfigStore::load_or_seed(Arc::new(SqliteConfigRepository::new(pool.clone())))
            .await
            .expect("failed to load config");
    let state = AppStateBuilder::new(
        StandartRandomProvider::new(),
        Arc::clone(&config_store),
        Arc::clone(&credentials_repo),
        pool,
    )
    .build()
    .await
    .expect("failed to build app state");

    match config_store.twitch() {
        Some(twitch) => {
            tracing::info!(
                "twitch config ready: client_id={}, broadcaster_id={}, redirect_uri={}, credentials_redirect_uri={}",
                twitch.client_id,
                twitch.broadcaster_id,
                twitch.redirect_uri,
                twitch.credentials_redirect_uri
            );
        }
        None => {
            tracing::info!("twitch config NOT configured: twitch login and ingress will not work");
        }
    }

    match config_store.vk_video_live() {
        Some(vk) => {
            tracing::info!(
                "vk video live config ready: client_id={}, channel_url={}, redirect_uri={}, credentials_redirect_uri={}",
                vk.client_id,
                vk.channel_url,
                vk.redirect_uri,
                vk.credentials_redirect_uri
            );
        }
        None => {
            tracing::info!("vk video live config NOT configured: vk ingress will not work");
        }
    }

    let mut platform_ids = Vec::new();
    if config_store.twitch().is_some() {
        platform_ids.push(PlatformId::TWITCH);
    }
    if config_store.vk_video_live().is_some() {
        platform_ids.push(PlatformId::VK_VIDEO_LIVE);
    }
    let platforms: &'static [PlatformId] = Box::leak(platform_ids.into_boxed_slice());
    let twitch_config = config_store.twitch().map(|t| Arc::new(t.clone()));
    let vk_config = config_store.vk_video_live().map(|t| Arc::new(t.clone()));
    let shutdown = CancellationToken::new();
    let build_ingress =
        move |platform: PlatformId,
              credentials: Arc<PlatformCredentialService<SqlitePlatformCredentialRepository>>,
              sink: EventSink,
              token: CancellationToken|
              -> Option<JoinHandle<()>> {
            match platform {
                PlatformId::TWITCH => match &twitch_config {
                    Some(config) => {
                        let config = Arc::clone(config);
                        Some(tokio::spawn(async move {
                            let service = TwitchPlatformService::new(config, credentials);
                            if let Err(e) = service.run(sink, token).await {
                                tracing::error!(
                                    "{} ingress stopped: {e}",
                                    service.platform().as_name()
                                );
                            }
                        }))
                    }
                    None => None,
                },
                PlatformId::VK_VIDEO_LIVE => match &vk_config {
                    Some(config) => {
                        let config = Arc::clone(config);
                        Some(tokio::spawn(async move {
                            let auth = Arc::new(VkVideoLiveAuthService::new(config, credentials));
                            let service = VkVideoLivePlatformService::new(auth);
                            if let Err(e) = service.run(sink, token).await {
                                tracing::error!(
                                    "{} ingress stopped: {e}",
                                    service.platform().as_name()
                                );
                            }
                        }))
                    }
                    None => None,
                },
                _ => None,
            }
        };
    let supervisor = IngressSupervisor::new(
        Arc::clone(&state.credentials),
        state.ingress.sink(),
        platforms,
    );
    tokio::spawn(supervisor.run(build_ingress, shutdown.clone()));
    let cors = match config_store.cors_origins() {
        Some(origins) => {
            let origins: Vec<HeaderValue> = origins.iter().filter_map(|o| o.parse().ok()).collect();
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        }
        None => CorsLayer::permissive(),
    };

    let spec = openapi();
    let app = api::router(state.clone())
        .merge(widget_api::router(state.clone()))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", spec.clone()))
        .merge(Redoc::with_url("/redoc", spec));

    let addr = format!("0.0.0.0:{}", config_store.port());
    let listener = TcpListener::bind(&addr).await.expect("failed to bind");
    info!("listening on http://{}", addr);

    start_background_tasks(&state, &shutdown);

    tokio::spawn(signal_listener(shutdown.clone()));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown.cancelled_owned())
        .await
        .expect("server failed");
}

fn start_background_tasks(state: &AppState, shutdown: &CancellationToken) {
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

    runtime::start_rule_pipeline(state, shutdown);
}

async fn signal_listener(token: CancellationToken) {
    let ctrl_c = async {
        ctrl_c().await.expect("failed to install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
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

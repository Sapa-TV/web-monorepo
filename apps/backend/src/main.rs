#![deny(clippy::exhaustive_structs)]
#![deny(clippy::new_ret_no_self)]

use std::future::pending;
use std::sync::Arc;
use std::time::Duration;

use axum::http::{HeaderValue, Method, header};
use backend::ApiDoc;
use backend::api;
use backend::config::store::ConfigStore;
use backend::db::inmemory_config::InMemoryConfigRepository;
use backend::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
use backend::ingress::PlatformService;
use backend::ingress::twitch::TwitchPlatformService;
use backend::random::StandartRandomProvider;
use backend::state::{AppQueueService, AppSessionService, AppState, AppStateBuilder};
use backend::widget_api;
use tokio::net::TcpListener;
use tokio::signal::ctrl_c;
use tokio::time;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use utoipa::OpenApi;
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

    let credentials_repo = Arc::new(InMemoryPlatformCredentialRepository::new());
    let config_store = ConfigStore::load_or_seed()
        .await
        .expect("failed to load config");
    let state = AppStateBuilder::new(
        StandartRandomProvider::new(),
        Arc::clone(&config_store),
        Arc::clone(&credentials_repo),
    )
    .build()
    .await
    .expect("failed to build app state");

    if let Some(twitch) = config_store.twitch() {
        let service =
            TwitchPlatformService::new(Arc::new(twitch.clone()), Arc::clone(&credentials_repo));
        let sink = state.ingress.sink();
        tracing::info!(
            "twitch config ready: client_id={}, broadcaster_id={}, redirect_uri={}",
            twitch.client_id,
            twitch.broadcaster_id,
            twitch.redirect_uri
        );
        tracing::info!("starting {} ingress", service.platform().as_name());
        tokio::spawn(async move {
            if let Err(e) = service.run(sink).await {
                tracing::error!("twitch ingress stopped: {e}");
            }
        });
    } else {
        tracing::info!(
            "twitch config NOT configured: twitch login and ingress will not work"
        );
    }
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

    let app = api::router(state.clone())
        .merge(widget_api::router(state.clone()))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .merge(Redoc::with_url("/redoc", ApiDoc::openapi()));

    let addr = format!("0.0.0.0:{}", config_store.port());
    let listener = TcpListener::bind(&addr).await.expect("failed to bind");
    info!("listening on http://{}", addr);

    start_background_tasks(&state);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server failed");
}

fn start_background_tasks(state: &AppState) {
    tokio::spawn(queue_timeout_task(Arc::clone(&state.queue_service)));
    tokio::spawn(queue_purge_task(
        Arc::clone(&state.queue_service),
        Arc::clone(&state.config),
    ));
    tokio::spawn(session_prune_task(
        Arc::clone(&state.session_service),
        Arc::clone(&state.config),
    ));
}

async fn shutdown_signal() {
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

    info!("shutdown signal received, draining connections");
}

async fn queue_timeout_task(queue_service: Arc<AppQueueService>) {
    loop {
        time::sleep(queue_service.timeout()).await;
        if let Err(e) = queue_service.mark_timed_out().await {
            tracing::error!("mark_timed_out failed: {e}");
        }
    }
}

async fn queue_purge_task(
    queue_service: Arc<AppQueueService>,
    config: Arc<ConfigStore<InMemoryConfigRepository>>,
) {
    loop {
        let interval = Duration::from_secs(config.queue_cleanup_interval_secs());
        time::sleep(interval).await;
        if let Err(e) = queue_service.purge_expired().await {
            tracing::error!("queue purge_expired failed: {e}");
        }
    }
}

async fn session_prune_task(
    session_service: Arc<AppSessionService>,
    config: Arc<ConfigStore<InMemoryConfigRepository>>,
) {
    loop {
        let interval = Duration::from_secs(config.sessions_cleanup_interval_secs());
        time::sleep(interval).await;
        if let Err(e) = session_service.prune_expired().await {
            tracing::error!("session prune_expired failed: {e}");
        }
    }
}

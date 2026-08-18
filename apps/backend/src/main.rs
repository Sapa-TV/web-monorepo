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
use backend::ingress::platform::EventSink;
use backend::ingress::supervisor::IngressSupervisor;
use backend::ingress::twitch::TwitchPlatformService;
use backend::platform::{PlatformCredentialService, PlatformId};
use backend::random::StandartRandomProvider;
use backend::runtime;
use backend::state::{AppQueueService, AppSessionService, AppState, AppStateBuilder};
use backend::widget_api;
use tokio::net::TcpListener;
use tokio::signal::ctrl_c;
use tokio::task::JoinHandle;
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

    let platforms: &'static [PlatformId] = match config_store.twitch() {
        Some(_) => &[PlatformId::TWITCH],
        None => &[],
    };
    let twitch_config = config_store.twitch().map(|t| Arc::new(t.clone()));
    let build_ingress =
        move |platform: PlatformId,
              credentials: Arc<PlatformCredentialService<InMemoryPlatformCredentialRepository>>,
              sink: EventSink|
              -> Option<JoinHandle<()>> {
            match platform {
                PlatformId::TWITCH => match &twitch_config {
                    Some(config) => {
                        let config = Arc::clone(config);
                        Some(tokio::spawn(async move {
                            let service = TwitchPlatformService::new(config, credentials);
                            if let Err(e) = service.run(sink).await {
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
    tokio::spawn(supervisor.run(build_ingress));
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

    runtime::start_rule_pipeline(state);
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

#![deny(clippy::exhaustive_structs)]
#![deny(clippy::new_ret_no_self)]
#![cfg_attr(dylint_lib = "new_returns_self", deny(new_returns_self))]

use std::future::pending;

use axum::http::{HeaderValue, Method, header};
use axum::middleware::from_fn_with_state;
use backend::api::{self, ApiDoc};
use backend::api::auth::require_auth;
use backend::config::Config;
use backend::ingress::PlatformService;
use backend::ingress::twitch::TwitchPlatformService;
use backend::random::StandartRandomProvider;
use backend::state::{AppState, AppStateBuilder};
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

    let config = Config::load();
    let state = AppStateBuilder::new(StandartRandomProvider::new(), &config)
        .build()
        .await
        .expect("failed to build app state");

    if let Some(twitch) = &config.twitch {
        let service = TwitchPlatformService::new(twitch.clone());
        let sink = state.ingress.sink();
        tracing::info!("starting {} ingress", service.kind().as_name());
        tokio::spawn(async move {
            if let Err(e) = service.run(sink).await {
                tracing::error!("twitch ingress stopped: {e}");
            }
        });
    }
    let cors = match state.config.cors_origins.as_deref() {
        Some(origins) => {
            let origins: Vec<HeaderValue> = origins.iter().filter_map(|o| o.parse().ok()).collect();
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        }
        None => CorsLayer::permissive(),
    };

    let app = api::router_with_auth(state.clone(), |router| {
        router.route_layer(from_fn_with_state(state.clone(), require_auth))
    })
    .layer(cors)
    .layer(TraceLayer::new_for_http())
    .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
    .merge(Redoc::with_url("/redoc", ApiDoc::openapi()));

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&addr).await.expect("failed to bind");
    info!("listening on http://{}", addr);

    tokio::spawn(timeout_task(state));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server failed");
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

async fn timeout_task(state: AppState) {
    let timeout = state.queue_service.timeout();
    let mut interval = time::interval(timeout);
    loop {
        interval.tick().await;
        if let Err(e) = state.queue_service.mark_timed_out().await {
            tracing::error!("mark_timed_out failed: {e}");
        }
        if let Err(e) = state.queue_service.purge_expired().await {
            tracing::error!("purge_expired failed: {e}");
        }
    }
}
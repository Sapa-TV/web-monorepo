#![deny(clippy::exhaustive_structs)]
#![deny(clippy::new_ret_no_self)]

mod background;
mod ingress_factory;
mod logging;

use std::sync::Arc;

use axum::http::{HeaderValue, Method, header};
use backend::api;
use backend::config::store::ConfigStore;
use backend::db::sqlite::config::SqliteConfigRepository;
use backend::db::sqlite::connect_from_env;
use backend::db::sqlite::platform_credential::SqlitePlatformCredentialRepository;
use backend::ingress::supervisor::IngressSupervisor;
use backend::openapi;
use backend::random::StandartRandomProvider;
use backend::state::AppStateBuilder;
use backend::widget_api;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing::warn;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

#[tokio::main]
async fn main() {
    let _log_guard = logging::init();

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

    ingress_factory::log_platform_config(&config_store);

    let shutdown = CancellationToken::new();
    let twitch_config = config_store.twitch().map(|t| Arc::new(t.clone()));
    let vk_config = config_store.vk_video_live().map(|t| Arc::new(t.clone()));
    let (platforms, build_ingress) = ingress_factory::ingress_factory(twitch_config, vk_config);
    let supervisor = IngressSupervisor::new(
        Arc::clone(&state.credentials),
        state.ingress.sink(),
        platforms,
    );
    tokio::spawn(supervisor.run(build_ingress, shutdown.clone()));

    let cors = match config_store.cors_origins() {
        Some(origins) => {
            let parsed: Vec<HeaderValue> = origins.iter().filter_map(|o| o.parse().ok()).collect();
            let skipped = origins.len() - parsed.len();
            if skipped > 0 {
                warn!("{skipped} invalid origin(s) in CORS_ORIGINS were skipped");
            }
            Some(
                CorsLayer::new()
                    .allow_origin(parsed)
                    .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
                    .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]),
            )
        }
        None => {
            warn!("CORS_ORIGINS is not set: cross-origin browser access is disabled");
            None
        }
    };

    let spec = openapi();
    let app = api::router(state.clone()).merge(widget_api::router(state.clone()));
    let app = match cors {
        Some(cors) => app.layer(cors),
        None => app,
    };
    let app = app
        .layer(TraceLayer::new_for_http())
        .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", spec.clone()))
        .merge(Redoc::with_url("/redoc", spec));

    let addr = format!("0.0.0.0:{}", config_store.port());
    let listener = TcpListener::bind(&addr).await.expect("failed to bind");
    info!("listening on http://{}", addr);

    background::start_background_tasks(&state, &shutdown);
    tokio::spawn(background::signal_listener(shutdown.clone()));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown.cancelled_owned())
        .await
        .expect("server failed");
}

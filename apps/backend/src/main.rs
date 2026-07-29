#![feature(sync_nonpoison)]
#![feature(nonpoison_mutex)]
#![deny(clippy::exhaustive_structs)]

mod api;
mod config;
mod db;
mod error;
mod event;
mod platform;
mod queue;
mod random;
mod roulette;
mod state;
#[cfg(test)]
mod test_fixtures;
mod user;

use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::ApiDoc;
use crate::config::Config;
use crate::random::StandartRandomProvider;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or(tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::default();
    let state = AppState::new(StandartRandomProvider, &config);
    let cors = CorsLayer::permissive();

    let app = api::router()
        .with_state(state.clone())
        .layer(cors)
        .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .nest_service("/", ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind to 0.0.0.0:3000");
    info!("listening on http://0.0.0.0:3000");

    tokio::spawn(timeout_task(state));

    axum::serve(listener, app).await.expect("server failed");
}

async fn timeout_task(state: AppState) {
    let timeout = state.queue_service.timeout();
    let mut interval = tokio::time::interval(timeout);
    loop {
        interval.tick().await;
        let _ = state.queue_service.mark_timed_out().await;
    }
}

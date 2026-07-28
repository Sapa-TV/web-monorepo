#![feature(sync_nonpoison)]
#![feature(nonpoison_mutex)]

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
mod user;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or(tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}

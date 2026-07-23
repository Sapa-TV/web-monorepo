#![feature(sync_nonpoison)]
#![feature(nonpoison_mutex)]

mod db;
mod error;
mod random;
mod roulette;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or(tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}

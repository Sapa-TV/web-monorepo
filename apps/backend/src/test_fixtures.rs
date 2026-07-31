#![cfg(test)]

use crate::config::Config;
use crate::random::StandartRandomProvider;
use crate::state::{AppState, AppStateBuilder};

pub async fn test_state() -> AppState {
    AppStateBuilder::new(StandartRandomProvider, &Config::test_config())
        .with_empty_repos()
        .build()
        .await
        .expect("failed to build test state")
}

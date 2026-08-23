use std::sync::Arc;

use tokio::select;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::actions::event::ActionEvent;
use crate::actions::executor::ActionExecutor;
use crate::consts::actions::BUS_CAPACITY;
use crate::rules::engine::RuleEngine;
use crate::state::AppState;

pub fn start_rule_pipeline(state: &AppState, shutdown: &CancellationToken) {
    let (tx, rx) = mpsc::channel::<ActionEvent>(BUS_CAPACITY);

    let twitch_config = state.config.twitch().map(|twitch| Arc::new(twitch.clone()));
    let executor = Arc::new(ActionExecutor::new(
        Arc::clone(&state.queue_service),
        Arc::clone(&state.user_service),
        state.twitch_api.clone(),
        twitch_config,
    ));

    let engine = RuleEngine::new(
        Arc::clone(&state.rule_service),
        Arc::clone(&state.action_service),
    );
    let event_rx = state.ingress.subscribe();

    let engine_token = shutdown.child_token();
    tokio::spawn(async move {
        select! {
            biased;
            _ = engine_token.cancelled() => {}
            _ = engine.run(event_rx, tx) => {}
        }
    });

    let executor_token = shutdown.child_token();
    tokio::spawn(async move {
        select! {
            biased;
            _ = executor_token.cancelled() => {}
            _ = executor.run(rx) => {}
        }
    });
}

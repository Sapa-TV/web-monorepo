use std::sync::Arc;

use tokio::sync::mpsc;

use crate::actions::event::ActionEvent;
use crate::actions::executor::ActionExecutor;
use crate::rules::engine::RuleEngine;
use crate::state::AppState;

const ACTION_BUS_CAPACITY: usize = 256;

pub fn start_rule_pipeline(state: &AppState) {
    let (tx, rx) = mpsc::channel::<ActionEvent>(ACTION_BUS_CAPACITY);

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
    tokio::spawn(async move { engine.run(event_rx, tx).await });
    tokio::spawn(async move { executor.run(rx).await });
}

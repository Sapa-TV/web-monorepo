use std::sync::Arc;

use tokio::select;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::actions::twitch_executor::TwitchActionExecutor;
use crate::actions::vk_video_live_executor::VkVideoLiveActionExecutor;
use crate::state::AppState;
use domain::actions::event::ActionEvent;
use domain::actions::executor::ActionExecutor;
use domain::actions::service::PlatformActionService;
use domain::rules::engine::RuleEngine;
use model::consts::actions::BUS_CAPACITY;

pub fn start_rule_pipeline(state: &AppState, shutdown: &CancellationToken) {
    let (tx, rx) = mpsc::channel::<ActionEvent>(BUS_CAPACITY);

    let mut platform_actions = PlatformActionService::new();
    if let (Some(config), Some(twitch_api)) = (
        state.config.twitch().map(|twitch| Arc::new(twitch.clone())),
        state.twitch_api.clone(),
    ) {
        platform_actions =
            platform_actions.with_twitch(TwitchActionExecutor::new(config, twitch_api));
    }
    if let Some(vk_api) = state.vk_api.clone() {
        platform_actions =
            platform_actions.with_vk_video_live(VkVideoLiveActionExecutor::new(vk_api));
    }

    let executor = Arc::new(ActionExecutor::new(
        Arc::clone(&state.queue_service),
        Arc::clone(&state.user_service),
        Arc::new(platform_actions),
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

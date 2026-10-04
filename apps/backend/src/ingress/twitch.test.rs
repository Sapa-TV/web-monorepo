use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
use crate::ingress::event::PlatformEventPayload;
use crate::platform::PlatformCredentialService;

fn test_service() -> TwitchPlatformService<InMemoryPlatformCredentialRepository> {
    let config = Arc::new(TwitchConfig::fixture());
    TwitchPlatformService::new(
        config,
        Arc::new(PlatformCredentialService::new(Arc::new(
            InMemoryPlatformCredentialRepository::new(),
        ))),
    )
}

#[tokio::test]
async fn run_returns_ok_immediately_when_shutdown_pre_cancelled() {
    let service = test_service();
    let (sink, _rx) = mpsc::channel::<PlatformEvent>(16);
    let shutdown = CancellationToken::new();
    shutdown.cancel();

    let result = timeout(Duration::from_secs(2), service.run(sink, shutdown))
        .await
        .expect("run must not hang on pre-cancelled shutdown");

    assert!(result.is_ok(), "cancelled run must be Ok, not an error");
}

#[test]
fn maps_chat_message_fields() {
    let event = chat_event_from(
        PlatformId::TWITCH,
        "msg-1",
        "4145994",
        "viewer32",
        "Hi chat",
    );
    assert_eq!(event.platform, PlatformId::TWITCH);
    assert_eq!(event.event_id, "msg-1");
    match &event.payload {
        PlatformEventPayload::ChatMessage(msg) => {
            assert_eq!(msg.user_id, "4145994");
            assert_eq!(msg.user_name, "viewer32");
            assert_eq!(msg.text, "Hi chat");
        }
        PlatformEventPayload::RewardRedemption(_) => unreachable!(),
    }
    assert_eq!(event.payload.type_name(), "chat_message");
}

#[test]
fn maps_reward_redemption_fields() {
    let data: ChannelPointsCustomRewardRedemptionAddV1Payload = serde_json::from_str(
        r##"{
                "id": "1234",
                "broadcaster_user_id": "1337",
                "broadcaster_user_login": "cool_user",
                "broadcaster_user_name": "Cool_User",
                "user_id": "9001",
                "user_login": "cooler_user",
                "user_name": "Cooler_User",
                "user_input": "pogchamp",
                "status": "unfulfilled",
                "reward": {
                    "id": "9001",
                    "title": "title",
                    "cost": 100,
                    "prompt": "reward prompt"
                },
                "redeemed_at": "2020-07-15T17:16:03.17106713Z"
            }"##,
    )
    .expect("payload should deserialize");
    let event = reward_redemption_event_from(PlatformId::TWITCH, &data);
    assert_eq!(event.event_id, "1234");
    match &event.payload {
        PlatformEventPayload::ChatMessage(_) => unreachable!(),
        PlatformEventPayload::RewardRedemption(red) => {
            assert_eq!(red.user_id, "9001");
            assert_eq!(red.user_name, "Cooler_User");
            assert_eq!(red.reward_id, "9001");
            assert_eq!(red.reward_title, "title");
            assert_eq!(red.reward_cost, 100);
            assert_eq!(red.user_input, "pogchamp");
            assert_eq!(red.status, "unfulfilled");
        }
    }
    assert_eq!(event.payload.type_name(), "reward_redemption");
}

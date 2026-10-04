use super::*;
use crate::ingress::event::PlatformEventPayload;
use crate::platform::PlatformId;

#[tokio::test]
async fn publish_delivers_to_subscriber() {
    let ingress = EventIngress::new();
    let mut rx = ingress.subscribe();
    let platform = PlatformId::TWITCH;
    let event = PlatformEvent::chat_message(
        platform,
        "msg-1",
        "1".to_string(),
        "viewer".to_string(),
        "hello".to_string(),
    );

    ingress.publish(event.clone()).await.unwrap();

    let received = rx.recv().await.unwrap();
    assert_eq!(received.platform, platform);
    assert_eq!(received.event_id, "msg-1");
    match &received.payload {
        PlatformEventPayload::ChatMessage(msg) => assert_eq!(msg.text, "hello"),
        PlatformEventPayload::RewardRedemption(_) => unreachable!(),
    }
}

#[tokio::test]
async fn publish_without_subscribers_is_ok() {
    let ingress = EventIngress::new();
    let event = PlatformEvent::chat_message(
        PlatformId::YOUTUBE,
        "msg-1",
        "1".to_string(),
        "viewer".to_string(),
        "hello".to_string(),
    );
    ingress.publish(event).await.unwrap();
}

#[tokio::test]
async fn duplicate_event_id_is_dropped() {
    let ingress = EventIngress::new();
    let mut rx = ingress.subscribe();
    let event = PlatformEvent::chat_message(
        PlatformId::TWITCH,
        "dup-1",
        "1".to_string(),
        "viewer".to_string(),
        "hello".to_string(),
    );
    let duplicate = event.clone();

    ingress.publish(event).await.unwrap();
    ingress.publish(duplicate).await.unwrap();

    let received = rx.recv().await.unwrap();
    assert_eq!(received.event_id, "dup-1");
    assert!(rx.try_recv().is_err(), "duplicate event must be dropped");
}

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc};

use crate::error::ingress::PlatformError;
use crate::ingress::event::{PlatformEvent, PlatformEventPayload};
use crate::ingress::platform::EventSink;
use crate::platform::PlatformId;

const CHANNEL_CAPACITY: usize = 64;
const DEDUP_WINDOW: usize = 1024;

type EventKey = (PlatformId, String);

#[non_exhaustive]
pub struct EventIngress {
    sink: mpsc::Sender<PlatformEvent>,
    out: broadcast::Sender<Arc<PlatformEvent>>,
}

impl EventIngress {
    pub fn new() -> Self {
        let (sink, mut rx) = mpsc::channel::<PlatformEvent>(CHANNEL_CAPACITY);
        let (out, _) = broadcast::channel::<Arc<PlatformEvent>>(CHANNEL_CAPACITY);
        let pump = out.clone();
        tokio::spawn(async move {
            let mut seen: HashSet<EventKey> = HashSet::with_capacity(DEDUP_WINDOW);
            let mut order: VecDeque<EventKey> = VecDeque::with_capacity(DEDUP_WINDOW);
            while let Some(event) = rx.recv().await {
                let key = (event.platform, event.event_id.clone());
                if !seen.insert(key.clone()) {
                    tracing::debug!(
                        platform = ?key.0,
                        event_id = %key.1,
                        "ingress: duplicate event dropped"
                    );
                    continue;
                }
                order.push_back(key);
                if order.len() > DEDUP_WINDOW
                    && let Some(evicted) = order.pop_front()
                {
                    seen.remove(&evicted);
                }
                let event = Arc::new(event);
                if pump.send(Arc::clone(&event)).is_err() {
                    tracing::debug!("ingress: no subscribers, dropping event");
                }
            }
        });
        Self { sink, out }
    }

    pub fn sink(&self) -> EventSink {
        self.sink.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<PlatformEvent>> {
        self.out.subscribe()
    }

    #[allow(dead_code)]
    pub async fn publish(&self, event: PlatformEvent) -> Result<(), PlatformError> {
        self.sink
            .send(event)
            .await
            .map_err(|_| PlatformError::Publish("sink closed".to_string()))
    }
}

impl Default for EventIngress {
    fn default() -> Self {
        Self::new()
    }
}

pub fn spawn_logging_handler(rx: broadcast::Receiver<Arc<PlatformEvent>>) {
    tokio::spawn(async move {
        let mut rx = rx;
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let payload_type = event.payload.type_name();
                    match &event.payload {
                        PlatformEventPayload::ChatMessage(msg) => {
                            tracing::info!(
                                platform = ?event.platform,
                                sent_at = ?event.sent_at,
                                payload_type,
                                user_id = %msg.user_id,
                                user_name = %msg.user_name,
                                text = %msg.text,
                                "ingress event received"
                            );
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!("ingress logging handler lagged, skipped {skipped} events");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
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
}

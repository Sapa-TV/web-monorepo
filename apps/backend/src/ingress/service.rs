use std::sync::Arc;

use tokio::sync::{broadcast, mpsc};

use crate::error::ingress::PlatformError;
use crate::ingress::event::{PlatformEvent, PlatformEventPayload};
use crate::ingress::platform::EventSink;

const CHANNEL_CAPACITY: usize = 64;

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
            while let Some(event) = rx.recv().await {
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
    use crate::ingress::event::{PlatformEventPayload, PlatformKind};

    #[tokio::test]
    async fn publish_delivers_to_subscriber() {
        let ingress = EventIngress::new();
        let mut rx = ingress.subscribe();
        let event = PlatformEvent::chat_message(
            PlatformKind::Twitch,
            "1".to_string(),
            "viewer".to_string(),
            "hello".to_string(),
        );

        ingress.publish(event.clone()).await.unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.platform, PlatformKind::Twitch);
        match &received.payload {
            PlatformEventPayload::ChatMessage(msg) => assert_eq!(msg.text, "hello"),
        }
    }

    #[tokio::test]
    async fn publish_without_subscribers_is_ok() {
        let ingress = EventIngress::new();
        let event = PlatformEvent::chat_message(
            PlatformKind::YouTube,
            "1".to_string(),
            "viewer".to_string(),
            "hello".to_string(),
        );
        ingress.publish(event).await.unwrap();
    }
}

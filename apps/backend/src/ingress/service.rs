use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc};

use crate::consts::ingress::{CHANNEL_CAPACITY, DEDUP_WINDOW};
use crate::error::ingress::PlatformError;
use crate::ingress::event::{PlatformEvent, PlatformEventPayload};
use crate::ingress::platform::EventSink;
use crate::platform::PlatformId;

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
                        PlatformEventPayload::RewardRedemption(red) => {
                            tracing::info!(
                                platform = ?event.platform,
                                sent_at = ?event.sent_at,
                                payload_type,
                                user_id = %red.user_id,
                                user_name = %red.user_name,
                                reward_title = %red.reward_title,
                                reward_cost = red.reward_cost,
                                status = %red.status,
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
#[path = "service.test.rs"]
mod tests;

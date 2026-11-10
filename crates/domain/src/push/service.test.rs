use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

use storage_inmemory::inmemory_push::InMemoryPushSubscriptionRepository;

use model::push::PushSubscription;
use model::push::repository::PushSubscriptionRepository;
use model::user::UserId;

use crate::push::service::{PushOutcome, PushPayload, PushSender, PushService};

struct FakeSender {
    outcomes: Mutex<VecDeque<PushOutcome>>,
}

impl FakeSender {
    fn new(outcomes: Vec<PushOutcome>) -> Self {
        Self {
            outcomes: Mutex::new(outcomes.into()),
        }
    }
}

impl PushSender for FakeSender {
    async fn send(
        &self,
        _subscription: &PushSubscription,
        _payload: &PushPayload,
    ) -> Result<PushOutcome, String> {
        Ok(self
            .outcomes
            .lock()
            .expect("lock")
            .pop_front()
            .unwrap_or(PushOutcome::Delivered))
    }
}

fn payload() -> PushPayload {
    PushPayload::new("title", "body", "https://sapa-tv.ru/", "tag")
}

#[tokio::test]
async fn subscribe_validates_endpoint() {
    let repo = Arc::new(InMemoryPushSubscriptionRepository::new());
    let service = PushService::new(Arc::clone(&repo), Arc::new(FakeSender::new(vec![])));

    let error = service
        .subscribe(UserId::new(1), "http://push.example/1", "key", "auth", None)
        .await
        .expect_err("http rejected");
    assert!(error.to_string().contains("https"));
}

#[tokio::test]
async fn broadcast_deletes_gone_and_counts_retry() {
    let repo = Arc::new(InMemoryPushSubscriptionRepository::new());
    let sender = Arc::new(FakeSender::new(vec![
        PushOutcome::Delivered,
        PushOutcome::Gone,
        PushOutcome::Retry,
    ]));
    let service = PushService::new(Arc::clone(&repo), sender);

    for index in 0..3 {
        service
            .subscribe(
                UserId::new(1),
                &format!("https://push.example/{index}"),
                "key",
                "auth",
                None,
            )
            .await
            .expect("subscribe");
    }

    let delivered = service.broadcast(&payload()).await.expect("broadcast");
    assert_eq!(delivered, 1);
    let remaining = repo.list_all().await.expect("list");
    assert_eq!(remaining.len(), 2);
    assert!(remaining.iter().any(|it| it.failure_count == 1));
}

#[tokio::test]
async fn unsubscribe_only_removes_own_endpoint() {
    let repo = Arc::new(InMemoryPushSubscriptionRepository::new());
    let service = PushService::new(Arc::clone(&repo), Arc::new(FakeSender::new(vec![])));

    service
        .subscribe(
            UserId::new(1),
            "https://push.example/1",
            "key",
            "auth",
            None,
        )
        .await
        .expect("subscribe");
    assert!(
        !service
            .unsubscribe(UserId::new(2), "https://push.example/1")
            .await
            .expect("unsubscribe")
    );
    assert!(
        service
            .unsubscribe(UserId::new(1), "https://push.example/1")
            .await
            .expect("unsubscribe")
    );
}

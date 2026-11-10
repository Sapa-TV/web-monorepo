use crate::inmemory_push::InMemoryPushSubscriptionRepository;

use model::push::NewPushSubscription;
use model::push::repository::PushSubscriptionRepository;
use model::user::UserId;

#[tokio::test]
async fn upsert_reuses_endpoint_and_moves_user() {
    let repo = InMemoryPushSubscriptionRepository::new();
    let first_user = UserId::new(1);
    let second_user = UserId::new(2);

    let first = repo
        .upsert(&NewPushSubscription::new(
            first_user,
            "https://push.example/1",
            "p256dh",
            "auth",
            Some("ua".to_string()),
        ))
        .await
        .expect("upsert");
    let second = repo
        .upsert(&NewPushSubscription::new(
            second_user,
            "https://push.example/1",
            "p256dh2",
            "auth2",
            None,
        ))
        .await
        .expect("upsert");

    assert_eq!(second.id, first.id);
    assert_eq!(second.user_id, second_user);
    assert_eq!(repo.list_all().await.expect("list").len(), 1);

    repo.record_failure(second.id).await.expect("failure");
    assert_eq!(
        repo.list_by_user(second_user).await.expect("list")[0].failure_count,
        1
    );
    repo.record_delivery(second.id).await.expect("delivery");
    assert_eq!(
        repo.list_by_user(second_user).await.expect("list")[0].failure_count,
        0
    );

    assert!(
        repo.delete_by_user_and_endpoint(second_user, "https://push.example/1")
            .await
            .expect("delete")
    );
    assert!(repo.list_all().await.expect("list").is_empty());
}

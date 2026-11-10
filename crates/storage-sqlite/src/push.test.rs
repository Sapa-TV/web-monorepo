use std::fs;

use model::push::NewPushSubscription;
use model::push::repository::PushSubscriptionRepository;
use model::user::UserId;
use model::user::repository::UserRepository;

use crate::push::SqlitePushSubscriptionRepository;
use crate::test_pool;
use crate::user::SqliteUserRepository;

async fn user_id(repo: &SqliteUserRepository) -> UserId {
    repo.create("guest").await.expect("create user").id
}

#[tokio::test]
async fn upsert_reuses_endpoint_and_moves_user() {
    let (pool, path) = test_pool().await;
    let users = SqliteUserRepository::new(pool.clone());
    let repo = SqlitePushSubscriptionRepository::new(pool.clone());
    let first_user = user_id(&users).await;
    let second_user = user_id(&users).await;

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
    assert_eq!(first.user_id, first_user);

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
    assert_eq!(second.p256dh, "p256dh2");

    repo.record_failure(second.id).await.expect("failure");
    let all = repo.list_all().await.expect("list");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].failure_count, 1);

    repo.record_delivery(second.id).await.expect("delivery");
    let all = repo.list_all().await.expect("list");
    assert_eq!(all[0].failure_count, 0);

    assert!(
        repo.delete_by_user_and_endpoint(second_user, "https://push.example/1")
            .await
            .expect("delete")
    );
    assert!(repo.list_all().await.expect("list").is_empty());

    drop(pool);
    fs::remove_file(path).ok();
}

#[tokio::test]
async fn merge_users_moves_subscriptions() {
    let (pool, path) = test_pool().await;
    let users = SqliteUserRepository::new(pool.clone());
    let repo = SqlitePushSubscriptionRepository::new(pool.clone());
    let source = user_id(&users).await;
    let target = user_id(&users).await;

    repo.upsert(&NewPushSubscription::new(
        source,
        "https://push.example/source",
        "p256dh",
        "auth",
        None,
    ))
    .await
    .expect("source subscription");
    repo.upsert(&NewPushSubscription::new(
        target,
        "https://push.example/target",
        "p256dh",
        "auth",
        None,
    ))
    .await
    .expect("target subscription");

    users.merge_users(source, target).await.expect("merge");

    let subscriptions = repo.list_by_user(target).await.expect("list");
    assert_eq!(subscriptions.len(), 2);

    drop(pool);
    fs::remove_file(path).ok();
}

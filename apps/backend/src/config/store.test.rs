use super::*;

use crate::db::inmemory_config::InMemoryConfigRepository;

fn test_store() -> (
    ConfigStore<InMemoryConfigRepository>,
    Arc<InMemoryConfigRepository>,
) {
    let repo = Arc::new(InMemoryConfigRepository::new());
    let store = ConfigStore::new(
        Arc::new(StaticConfig::test_config()),
        RuntimeConfig::test_runtime("secret"),
        Arc::clone(&repo),
    );
    (store, repo)
}

#[test]
fn accessors_return_configured_values() {
    let (store, _) = test_store();
    assert_eq!(store.widget_access_key(), "secret");
    assert_eq!(store.queue_default_limit(), 20);
    assert_eq!(store.session_ttl_secs(), 24 * 60 * 60);
    assert_eq!(store.port(), 3000);
    assert!(!store.cookie_secure());
    assert_eq!(store.admin_twitch_id(), None);
    assert_eq!(store.cors_origins(), None);
    assert!(store.twitch().is_none());
}

#[test]
fn admin_twitch_id_comes_from_twitch_broadcaster_id() {
    let repo = Arc::new(InMemoryConfigRepository::new());
    let mut twitch = TwitchConfig::fixture();
    twitch.broadcaster_id = "42".to_string();
    let static_cfg = Arc::new(StaticConfig::new(
        3000,
        None,
        false,
        Some(Arc::new(twitch)),
        None,
        None,
        None,
    ));
    let store = ConfigStore::new(
        static_cfg,
        RuntimeConfig::test_runtime("secret"),
        Arc::clone(&repo),
    );
    assert_eq!(store.admin_twitch_id(), Some("42"));
}

#[tokio::test]
async fn update_runtime_persists_and_is_visible() {
    let (store, repo) = test_store();
    let mut next = RuntimeConfig::test_runtime("other");
    next.session.ttl_secs = 42;

    store.update_runtime(next.clone()).await.unwrap();

    assert_eq!(store.source().read().session.ttl_secs, 42);
    assert_eq!(store.widget_access_key(), "other");
    assert_eq!(repo.load().await.unwrap().unwrap(), next);
}

#[tokio::test]
async fn update_runtime_invalid_not_applied() {
    let (store, repo) = test_store();

    let err = store
        .update_runtime(RuntimeConfig::default())
        .await
        .unwrap_err();

    assert!(matches!(err, ConfigError::InvalidWidgetAccessKey));
    assert!(repo.load().await.unwrap().is_none());
    assert_eq!(store.widget_access_key(), "secret");
}

#[tokio::test]
async fn rotate_access_key_changes_only_key() {
    let (store, repo) = test_store();

    store.rotate_widget_access_key("rotated").await.unwrap();

    assert_eq!(store.source().read().widget_access_key, "rotated");
    assert_eq!(store.source().read().session.ttl_secs, 24 * 60 * 60);
    assert_eq!(store.source().read().queue.default_limit, 20);
    assert_eq!(
        repo.load().await.unwrap().unwrap().widget_access_key,
        "rotated"
    );
}

#[tokio::test]
async fn rotate_access_key_empty_rejected() {
    let (store, repo) = test_store();

    let err = store.rotate_widget_access_key("").await.unwrap_err();

    assert!(matches!(err, ConfigError::InvalidWidgetAccessKey));
    assert!(repo.load().await.unwrap().is_none());
    assert_eq!(store.widget_access_key(), "secret");
}

#[tokio::test]
async fn rotate_access_key_generated_persists_and_is_visible() {
    let (store, repo) = test_store();

    let key = store.rotate_widget_access_key_generated().await.unwrap();

    assert!(!key.is_empty());
    assert_eq!(store.source().read().widget_access_key, key);
    assert_eq!(store.widget_access_key(), key);
    assert_eq!(repo.load().await.unwrap().unwrap().widget_access_key, key);
}

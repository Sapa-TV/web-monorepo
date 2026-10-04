use std::path::PathBuf;

use crate::db::sqlite::test_pool;

use super::*;

async fn repo() -> (SqliteConfigRepository, PathBuf) {
    let (pool, path) = test_pool().await;
    (SqliteConfigRepository::new(pool), path)
}

#[tokio::test]
async fn load_on_empty_repo_is_none() {
    let (repo, _path) = repo().await;
    assert!(repo.load().await.unwrap().is_none());
}

#[tokio::test]
async fn save_then_load_roundtrip() {
    let (repo, _path) = repo().await;
    let cfg = RuntimeConfig::test_runtime("test-key");

    repo.save(&cfg).await.unwrap();

    assert_eq!(repo.load().await.unwrap(), Some(cfg));
}

#[tokio::test]
async fn save_overwrites_previous() {
    let (repo, _path) = repo().await;

    repo.save(&RuntimeConfig::test_runtime("first"))
        .await
        .unwrap();
    repo.save(&RuntimeConfig::test_runtime("second"))
        .await
        .unwrap();

    let loaded = repo.load().await.unwrap().unwrap();
    assert_eq!(loaded.widget_access_key, "second");
}

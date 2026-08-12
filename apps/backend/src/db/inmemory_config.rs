use std::sync::nonpoison::Mutex;

use crate::config::repository::ConfigRepository;
use crate::config::runtime::RuntimeConfig;
use crate::error::RepositoryError;

#[non_exhaustive]
pub struct InMemoryConfigRepository {
    config: Mutex<Option<RuntimeConfig>>,
}

impl InMemoryConfigRepository {
    pub fn new() -> Self {
        Self {
            config: Mutex::new(None),
        }
    }
}

impl Default for InMemoryConfigRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigRepository for InMemoryConfigRepository {
    async fn load(&self) -> Result<Option<RuntimeConfig>, RepositoryError> {
        Ok(self.config.lock().clone())
    }

    async fn save(&self, config: &RuntimeConfig) -> Result<(), RepositoryError> {
        *self.config.lock() = Some(config.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn load_on_empty_repo_is_none() {
        let repo = InMemoryConfigRepository::new();
        assert!(repo.load().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn save_then_load_roundtrip() {
        let repo = InMemoryConfigRepository::new();
        let cfg = RuntimeConfig::test_runtime("test-key");

        repo.save(&cfg).await.unwrap();

        let loaded = repo.load().await.unwrap().unwrap();
        assert_eq!(loaded, cfg);
    }

    #[tokio::test]
    async fn save_overwrites_previous() {
        let repo = InMemoryConfigRepository::new();
        let first = RuntimeConfig::test_runtime("first");
        let second = RuntimeConfig::test_runtime("second");

        repo.save(&first).await.unwrap();
        repo.save(&second).await.unwrap();

        let loaded = repo.load().await.unwrap().unwrap();
        assert_eq!(loaded.access_key, "second");
    }

    #[tokio::test]
    async fn concurrent_saves_are_serialized() {
        use std::sync::Arc;

        let repo = Arc::new(InMemoryConfigRepository::new());
        let cfg_a = RuntimeConfig::test_runtime("a");
        let cfg_b = RuntimeConfig::test_runtime("b");

        let (repo_a, repo_b) = (Arc::clone(&repo), Arc::clone(&repo));
        let handle_a = tokio::spawn(async move { repo_a.save(&cfg_a).await });
        let handle_b = tokio::spawn(async move { repo_b.save(&cfg_b).await });

        handle_a.await.unwrap().unwrap();
        handle_b.await.unwrap().unwrap();

        let key = repo.load().await.unwrap().unwrap().access_key;
        assert!(key == "a" || key == "b");
    }
}

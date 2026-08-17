use std::sync::Arc;
use std::sync::nonpoison::{RwLock, RwLockReadGuard};

use crate::config::repository::ConfigRepository;
use crate::config::runtime::RuntimeConfig;
use crate::config::static_config::StaticConfig;
use crate::config::twitch::TwitchConfig;
use crate::db::inmemory_config::InMemoryConfigRepository;
use crate::error::ConfigError;
use crate::random::generate_secret;

#[derive(Clone)]
#[non_exhaustive]
pub struct SharedSettings(Arc<RwLock<RuntimeConfig>>);

impl SharedSettings {
    pub fn read(&self) -> RwLockReadGuard<'_, RuntimeConfig> {
        self.0.read()
    }

    #[cfg(test)]
    pub fn test_new(runtime: RuntimeConfig) -> Self {
        Self(Arc::new(RwLock::new(runtime)))
    }
}

#[non_exhaustive]
pub struct ConfigStore<R: ConfigRepository> {
    static_cfg: Arc<StaticConfig>,
    runtime_cfg: Arc<RwLock<RuntimeConfig>>,
    repo: Arc<R>,
}

impl<R: ConfigRepository> ConfigStore<R> {
    pub fn new(static_cfg: Arc<StaticConfig>, runtime_cfg: RuntimeConfig, repo: Arc<R>) -> Self {
        Self {
            static_cfg,
            runtime_cfg: Arc::new(RwLock::new(runtime_cfg)),
            repo,
        }
    }

    pub fn source(&self) -> SharedSettings {
        SharedSettings(Arc::clone(&self.runtime_cfg))
    }

    pub fn widget_access_key(&self) -> String {
        self.runtime_cfg.read().widget_access_key.clone()
    }

    pub fn queue_default_limit(&self) -> usize {
        self.runtime_cfg.read().queue_default_limit
    }

    pub fn session_ttl_secs(&self) -> u64 {
        self.runtime_cfg.read().session_ttl_secs
    }

    pub fn roulette_timeout_secs(&self) -> u64 {
        self.runtime_cfg.read().roulette_timeout_secs
    }

    pub fn retention_secs(&self) -> u64 {
        self.runtime_cfg.read().retention_secs
    }

    pub fn queue_cleanup_interval_secs(&self) -> u64 {
        self.runtime_cfg.read().queue_cleanup_interval_secs
    }

    pub fn sessions_cleanup_interval_secs(&self) -> u64 {
        self.runtime_cfg.read().sessions_cleanup_interval_secs
    }

    pub fn port(&self) -> u16 {
        self.static_cfg.port
    }

    pub fn cookie_secure(&self) -> bool {
        self.static_cfg.cookie_secure
    }

    pub fn admin_twitch_id(&self) -> Option<&str> {
        self.static_cfg
            .twitch
            .as_deref()
            .map(|twitch| twitch.broadcaster_id.as_str())
    }

    pub fn cors_origins(&self) -> Option<&[String]> {
        self.static_cfg.cors_origins.as_deref()
    }

    pub fn twitch(&self) -> Option<&TwitchConfig> {
        self.static_cfg.twitch.as_deref()
    }

    pub async fn update_runtime(&self, next: RuntimeConfig) -> Result<(), ConfigError> {
        next.validate()?;
        self.repo.save(&next).await?;
        *self.runtime_cfg.write() = next;
        Ok(())
    }

    async fn rotate_widget_access_key(&self, key: &str) -> Result<(), ConfigError> {
        let mut next = self.runtime_cfg.read().clone();
        next.widget_access_key = key.to_string();
        next.validate()?;
        self.repo.save(&next).await?;
        *self.runtime_cfg.write() = next;
        Ok(())
    }

    pub async fn rotate_widget_access_key_generated(&self) -> Result<String, ConfigError> {
        let key = generate_secret();
        self.rotate_widget_access_key(&key).await?;
        Ok(key)
    }
}

impl ConfigStore<InMemoryConfigRepository> {
    pub async fn load_or_seed() -> Result<Arc<Self>, ConfigError> {
        let (static_cfg, file_seed) = StaticConfig::load();
        let repo = Arc::new(InMemoryConfigRepository::new());
        let runtime = match repo.load().await? {
            Some(runtime) => runtime,
            None => {
                let mut seed = file_seed.unwrap_or_default();
                seed.widget_access_key = generate_secret();
                repo.save(&seed).await?;
                seed
            }
        };
        Ok(Arc::new(Self::new(Arc::new(static_cfg), runtime, repo)))
    }
}

#[cfg(test)]
mod tests {
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
        let static_cfg = Arc::new(StaticConfig {
            port: 3000,
            cors_origins: None,
            cookie_secure: false,
            twitch: Some(Arc::new(TwitchConfig {
                client_id: "cid".to_string(),
                client_secret: "cs".to_string(),
                broadcaster_id: "42".to_string(),
                redirect_uri: "https://localhost/cb".to_string(),
                credentials_redirect_uri: "https://localhost/creds/cb".to_string(),
                csrf_ttl_secs: 600,
            })),
        });
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
        next.session_ttl_secs = 42;

        store.update_runtime(next.clone()).await.unwrap();

        assert_eq!(store.source().read().session_ttl_secs, 42);
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
        assert_eq!(store.source().read().session_ttl_secs, 24 * 60 * 60);
        assert_eq!(store.source().read().queue_default_limit, 20);
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
}

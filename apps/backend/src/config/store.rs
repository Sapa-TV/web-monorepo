use std::sync::Arc;
use std::sync::nonpoison::{RwLock, RwLockReadGuard};

use crate::config::repository::ConfigRepository;
use crate::config::runtime::RuntimeConfig;
use crate::config::static_config::StaticConfig;
use crate::config::twitch::TwitchConfig;
use crate::config::vk_video_live::VkVideoLiveConfig;
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
        self.runtime_cfg.read().queue.default_limit
    }

    pub fn session_ttl_secs(&self) -> u64 {
        self.runtime_cfg.read().session.ttl_secs
    }

    pub fn roulette_timeout_secs(&self) -> u64 {
        self.runtime_cfg.read().roulette.timeout_secs
    }

    pub fn retention_secs(&self) -> u64 {
        self.runtime_cfg.read().queue.retention_secs
    }

    pub fn queue_cleanup_interval_secs(&self) -> u64 {
        self.runtime_cfg.read().queue.cleanup_interval_secs
    }

    pub fn sessions_cleanup_interval_secs(&self) -> u64 {
        self.runtime_cfg.read().session.cleanup_interval_secs
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

    pub fn admin_twitch_ids(&self) -> Option<&[String]> {
        self.static_cfg.admin_twitch_ids.as_deref()
    }

    pub fn cors_origins(&self) -> Option<&[String]> {
        self.static_cfg.cors_origins.as_deref()
    }

    pub fn twitch(&self) -> Option<&TwitchConfig> {
        self.static_cfg.twitch.as_deref()
    }

    pub fn vk_video_live(&self) -> Option<&VkVideoLiveConfig> {
        self.static_cfg.vk_video_live.as_deref()
    }

    pub fn google_service_account_key_base64(&self) -> Option<&str> {
        self.static_cfg.google_service_account_key_base64.as_deref()
    }

    pub fn sheets_spreadsheet_id(&self) -> String {
        self.runtime_cfg.read().sheets.spreadsheet_id.clone()
    }

    pub async fn set_sheets_spreadsheet_id(&self, spreadsheet_id: &str) -> Result<(), ConfigError> {
        let mut next = self.runtime_cfg.read().clone();
        next.sheets.spreadsheet_id = spreadsheet_id.to_string();
        self.update_runtime(next).await
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

impl<K: ConfigRepository> ConfigStore<K> {
    pub async fn load_or_seed(repo: Arc<K>) -> Result<Arc<Self>, ConfigError> {
        let (static_cfg, file_seed) = StaticConfig::load();
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
#[path = "store.test.rs"]
mod tests;

use std::sync::Arc;

use crate::config::static_config::StaticConfig;
use crate::config::twitch::TwitchConfig;
use crate::config::vk_id::VkIdConfig;
use crate::config::vk_video_live::VkVideoLiveConfig;
use domain::error::SheetsError;
use domain::random::generate_secret;
use domain::sheets::settings::SheetsSettings;
use model::config::ConfigError;
use model::config::repository::ConfigRepository;
use model::config::runtime::RuntimeConfig;

pub use domain::config::settings::SharedSettings;

#[non_exhaustive]
pub struct ConfigStore<R: ConfigRepository> {
    static_cfg: Arc<StaticConfig>,
    settings: SharedSettings,
    repo: Arc<R>,
}

impl<R: ConfigRepository> ConfigStore<R> {
    pub fn new(static_cfg: Arc<StaticConfig>, runtime_cfg: RuntimeConfig, repo: Arc<R>) -> Self {
        Self {
            static_cfg,
            settings: SharedSettings::new(runtime_cfg),
            repo,
        }
    }

    pub fn source(&self) -> SharedSettings {
        self.settings.clone()
    }

    pub fn widget_access_key(&self) -> String {
        self.settings.read().widget_access_key.clone()
    }

    pub fn queue_default_limit(&self) -> usize {
        self.settings.read().queue.default_limit
    }

    pub fn session_ttl_secs(&self) -> u64 {
        self.settings.read().session.ttl_secs
    }

    pub fn roulette_timeout_secs(&self) -> u64 {
        self.settings.read().roulette.timeout_secs
    }

    pub fn retention_secs(&self) -> u64 {
        self.settings.read().queue.retention_secs
    }

    pub fn queue_cleanup_interval_secs(&self) -> u64 {
        self.settings.read().queue.cleanup_interval_secs
    }

    pub fn sessions_cleanup_interval_secs(&self) -> u64 {
        self.settings.read().session.cleanup_interval_secs
    }

    pub fn port(&self) -> u16 {
        self.static_cfg.port
    }

    pub fn cookie_secure(&self) -> bool {
        self.static_cfg.cookie_secure
    }

    pub fn guest_session_ttl_secs(&self) -> u64 {
        self.static_cfg.guest_session_ttl_secs
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

    pub fn vk_id(&self) -> Option<&VkIdConfig> {
        self.static_cfg.vk_id.as_deref()
    }

    pub fn vk_video_live(&self) -> Option<&VkVideoLiveConfig> {
        self.static_cfg.vk_video_live.as_deref()
    }

    pub fn google_service_account_key_base64(&self) -> Option<&str> {
        self.static_cfg.google_service_account_key_base64.as_deref()
    }

    pub fn srs_hook_secret(&self) -> Option<&str> {
        self.static_cfg.srs_hook_secret.as_deref()
    }

    pub fn srs_stream_key(&self) -> Option<&str> {
        self.static_cfg.srs_stream_key.as_deref()
    }

    pub fn srs_api_url(&self) -> Option<&str> {
        self.static_cfg.srs_api_url.as_deref()
    }

    pub fn site_url(&self) -> &str {
        &self.static_cfg.site_url
    }

    pub fn vapid_private_key(&self) -> Option<&str> {
        self.static_cfg.vapid_private_key.as_deref()
    }

    pub fn vapid_public_key(&self) -> Option<&str> {
        self.static_cfg.vapid_public_key.as_deref()
    }

    pub fn vapid_subject(&self) -> &str {
        &self.static_cfg.vapid_subject
    }

    pub fn push_enabled(&self) -> bool {
        self.static_cfg
            .vapid_private_key
            .as_deref()
            .is_some_and(|key| !key.is_empty())
            && self
                .static_cfg
                .vapid_public_key
                .as_deref()
                .is_some_and(|key| !key.is_empty())
    }

    pub fn sheets_spreadsheet_id(&self) -> String {
        self.settings.read().sheets.spreadsheet_id.clone()
    }

    pub async fn set_sheets_spreadsheet_id(&self, spreadsheet_id: &str) -> Result<(), ConfigError> {
        let mut next = self.settings.read();
        next.sheets.spreadsheet_id = spreadsheet_id.to_string();
        self.update_runtime(next).await
    }

    pub async fn update_runtime(&self, next: RuntimeConfig) -> Result<(), ConfigError> {
        next.validate()?;
        self.repo.save(&next).await?;
        self.settings.set(next);
        Ok(())
    }

    async fn rotate_widget_access_key(&self, key: &str) -> Result<(), ConfigError> {
        let mut next = self.settings.read();
        next.widget_access_key = key.to_string();
        next.validate()?;
        self.repo.save(&next).await?;
        self.settings.set(next);
        Ok(())
    }

    pub async fn rotate_widget_access_key_generated(&self) -> Result<String, ConfigError> {
        let key = generate_secret();
        self.rotate_widget_access_key(&key).await?;
        Ok(key)
    }
}

impl<K: ConfigRepository> SheetsSettings for ConfigStore<K> {
    fn sheets_spreadsheet_id(&self) -> String {
        ConfigStore::sheets_spreadsheet_id(self)
    }

    async fn set_sheets_spreadsheet_id(&self, spreadsheet_id: &str) -> Result<(), SheetsError> {
        ConfigStore::set_sheets_spreadsheet_id(self, spreadsheet_id)
            .await
            .map_err(|e| SheetsError::Config(e.to_string()))
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

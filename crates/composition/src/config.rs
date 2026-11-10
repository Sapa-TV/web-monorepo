pub mod static_config;
pub mod store;
pub mod twitch;
pub mod vk_id;
pub mod vk_video_live;

use model::config::ConfigError;

pub use model::config::{ConfigRepository, RuntimeConfig, repository, runtime};
pub use static_config::StaticConfig;
pub use store::{ConfigStore, SharedSettings};
pub use twitch::TwitchConfig;
pub use vk_id::VkIdConfig;
pub use vk_video_live::VkVideoLiveConfig;

pub(crate) const DEFAULT_CSRF_TTL_SECS: u64 = 600;

pub(crate) fn default_csrf_ttl_secs() -> u64 {
    DEFAULT_CSRF_TTL_SECS
}

pub(crate) fn validate_required(fields: &[(&'static str, &str)]) -> Result<(), ConfigError> {
    for (name, value) in fields {
        if value.is_empty() {
            return Err(ConfigError::MissingField { field: name });
        }
    }
    Ok(())
}

pub(crate) fn validate_csrf_ttl(secs: u64) -> Result<(), ConfigError> {
    if secs == 0 {
        return Err(ConfigError::InvalidCsrfTtl);
    }
    Ok(())
}

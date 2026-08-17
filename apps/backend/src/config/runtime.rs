use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(default)]
pub struct RuntimeConfig {
    pub widget_access_key: String,
    pub roulette_timeout_secs: u64,
    pub retention_secs: u64,
    pub queue_cleanup_interval_secs: u64,
    pub sessions_cleanup_interval_secs: u64,
    pub queue_default_limit: usize,
    pub session_ttl_secs: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            widget_access_key: String::new(),
            roulette_timeout_secs: 10,
            retention_secs: 24 * 60 * 60,
            queue_cleanup_interval_secs: 60 * 60,
            sessions_cleanup_interval_secs: 60 * 60,
            queue_default_limit: 20,
            session_ttl_secs: 24 * 60 * 60,
        }
    }
}

impl RuntimeConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.widget_access_key.is_empty() {
            return Err(ConfigError::InvalidWidgetAccessKey);
        }
        for (field, value) in [
            ("roulette_timeout_secs", self.roulette_timeout_secs),
            ("retention_secs", self.retention_secs),
            (
                "queue_cleanup_interval_secs",
                self.queue_cleanup_interval_secs,
            ),
            (
                "sessions_cleanup_interval_secs",
                self.sessions_cleanup_interval_secs,
            ),
            ("queue_default_limit", self.queue_default_limit as u64),
            ("session_ttl_secs", self.session_ttl_secs),
        ] {
            if value == 0 {
                return Err(ConfigError::InvalidValue { field });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
impl RuntimeConfig {
    pub fn test_runtime(widget_access_key: &str) -> Self {
        Self {
            widget_access_key: widget_access_key.to_string(),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_empty_widget_access_key() {
        let cfg = RuntimeConfig::default();
        assert!(cfg.widget_access_key.is_empty());
        assert!(matches!(
            cfg.validate(),
            Err(ConfigError::InvalidWidgetAccessKey)
        ));
    }

    #[test]
    fn valid_config_accepted() {
        assert!(RuntimeConfig::test_runtime("secret").validate().is_ok());
    }

    #[test]
    fn zero_values_rejected() {
        let base = RuntimeConfig::test_runtime("secret");
        for (field, value) in [
            ("roulette_timeout_secs", 0_u64),
            ("retention_secs", 0_u64),
            ("queue_cleanup_interval_secs", 0_u64),
            ("sessions_cleanup_interval_secs", 0_u64),
            ("session_ttl_secs", 0_u64),
        ] {
            let mut cfg = base.clone();
            apply_u64(&mut cfg, field, value);
            assert!(
                matches!(
                    cfg.validate(),
                    Err(ConfigError::InvalidValue { field: actual }) if actual == field
                ),
                "expected InvalidValue for {field}"
            );
        }

        let mut cfg = base.clone();
        cfg.queue_default_limit = 0;
        assert!(matches!(
            cfg.validate(),
            Err(ConfigError::InvalidValue {
                field: "queue_default_limit"
            })
        ));
    }

    #[test]
    fn serde_roundtrip() {
        let cfg = RuntimeConfig::test_runtime("secret");
        let json = serde_json::to_string(&cfg).unwrap();
        let decoded: RuntimeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, cfg);
    }

    #[test]
    fn serde_unknown_field_ignored() {
        let json = r#"{"widget_access_key":"secret","unknown_field":42}"#;
        let cfg: RuntimeConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.widget_access_key, "secret");
    }

    fn apply_u64(cfg: &mut RuntimeConfig, field: &str, value: u64) {
        match field {
            "roulette_timeout_secs" => cfg.roulette_timeout_secs = value,
            "retention_secs" => cfg.retention_secs = value,
            "queue_cleanup_interval_secs" => cfg.queue_cleanup_interval_secs = value,
            "sessions_cleanup_interval_secs" => cfg.sessions_cleanup_interval_secs = value,
            "session_ttl_secs" => cfg.session_ttl_secs = value,
            _ => unreachable!(),
        }
    }
}

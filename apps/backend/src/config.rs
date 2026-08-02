#[derive(Clone)]
#[non_exhaustive]
pub struct Config {
    pub roulette_timeout_secs: u64,
    pub retention_secs: u64,
    pub queue_default_limit: usize,
    pub port: u16,
    pub access_key: String,
    pub cors_origins: Option<Vec<String>>,
}

impl Config {
    pub fn load() -> Self {
        if let Err(e) = dotenvy::dotenv() {
            tracing::warn!("failed to load .env: {e}");
        }
        Self {
            roulette_timeout_secs: 10,
            retention_secs: 24 * 60 * 60,
            queue_default_limit: 20,
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3000),
            access_key: std::env::var("ACCESS_KEY")
                .ok()
                .filter(|k| !k.is_empty())
                .expect("ACCESS_KEY env var must be set"),
            cors_origins: std::env::var("CORS_ORIGINS")
                .ok()
                .map(|v| {
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .filter(|v: &Vec<String>| !v.is_empty()),
        }
    }
}

#[cfg(test)]
impl Config {
    pub fn test_config() -> Self {
        Self {
            roulette_timeout_secs: 0,
            retention_secs: 24 * 60 * 60,
            queue_default_limit: 20,
            port: 3000,
            access_key: "test-key".to_string(),
            cors_origins: None,
        }
    }
}

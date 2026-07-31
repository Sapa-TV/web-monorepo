#[derive(Clone)]
#[non_exhaustive]
pub struct Config {
    pub roulette_timeout_secs: u64,
    pub port: u16,
    pub access_key: String,
}

impl Config {
    pub fn load() -> Self {
        if let Err(e) = dotenvy::dotenv() {
            tracing::warn!("failed to load .env: {e}");
        }
        Self {
            roulette_timeout_secs: 10,
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3000),
            access_key: std::env::var("ACCESS_KEY")
                .ok()
                .filter(|k| !k.is_empty())
                .expect("ACCESS_KEY env var must be set"),
        }
    }
}

#[cfg(test)]
impl Config {
    pub fn test_config() -> Self {
        Self {
            roulette_timeout_secs: 10,
            port: 3000,
            access_key: "test-key".to_string(),
        }
    }
}

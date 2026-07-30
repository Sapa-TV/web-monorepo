#[derive(Clone)]
#[non_exhaustive]
pub struct Config {
    pub roulette_timeout_secs: u64,
    pub port: u16,
    pub access_key: String,
}

impl Config {
    pub fn load() -> Self {
        Self {
            roulette_timeout_secs: 10,
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3000),
            access_key: std::env::var("ACCESS_KEY").unwrap_or_default(),
        }
    }
}

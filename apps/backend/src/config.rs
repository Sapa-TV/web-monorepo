#[non_exhaustive]
pub struct Config {
    pub roulette_timeout_secs: u64,
}

impl Config {
    pub const fn default() -> Self {
        Self {
            roulette_timeout_secs: 10,
        }
    }
}

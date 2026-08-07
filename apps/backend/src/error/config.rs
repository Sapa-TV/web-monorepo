use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("twitch config field `{field}` is required")]
    MissingField { field: &'static str },
    #[error("twitch config `csrf_ttl_secs` must be greater than zero")]
    InvalidCsrfTtl,
}

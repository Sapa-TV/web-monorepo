use thiserror::Error;

use super::RepositoryError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("twitch config field `{field}` is required")]
    MissingField { field: &'static str },
    #[error("twitch config `csrf_ttl_secs` must be greater than zero")]
    InvalidCsrfTtl,
    #[error("runtime config must not be empty")]
    InvalidAccessKey,
    #[error("runtime config field `{field}` must be greater than zero")]
    InvalidValue { field: &'static str },
    #[error("config repository: {0}")]
    Repo(#[from] RepositoryError),
}

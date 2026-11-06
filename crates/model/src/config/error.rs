use thiserror::Error;

use crate::error::RepositoryError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("twitch config field `{field}` is required")]
    MissingField { field: &'static str },
    #[error("twitch config `csrf_ttl_secs` must be greater than zero")]
    InvalidCsrfTtl,
    #[error("runtime config widget access key must not be empty")]
    InvalidWidgetAccessKey,
    #[error("runtime config field `{field}` must be greater than zero")]
    InvalidValue { field: &'static str },
    #[error("config repository: {0}")]
    Repo(#[from] RepositoryError),
}

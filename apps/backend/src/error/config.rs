use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use super::RepositoryError;
use super::api::ApiError;

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

impl From<ConfigError> for ApiError {
    fn from(e: ConfigError) -> Self {
        match e {
            ConfigError::Repo(re) => ApiError::from(re),
            ConfigError::MissingField { .. }
            | ConfigError::InvalidCsrfTtl
            | ConfigError::InvalidWidgetAccessKey
            | ConfigError::InvalidValue { .. } => {
                ApiError::new(StatusCode::BAD_REQUEST, e.to_string())
            }
        }
    }
}

impl IntoResponse for ConfigError {
    fn into_response(self) -> Response {
        ApiError::from(self).into_response()
    }
}

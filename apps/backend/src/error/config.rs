use axum::http::StatusCode;

use super::api::ApiError;

pub use model::config::ConfigError;

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
            _ => ApiError::new(StatusCode::BAD_REQUEST, e.to_string()),
        }
    }
}

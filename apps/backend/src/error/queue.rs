use super::api::ApiError;

pub use domain::error::queue::*;

impl From<QueueServiceError> for ApiError {
    fn from(e: QueueServiceError) -> Self {
        match e {
            QueueServiceError::Repo(re) => ApiError::from(re),
            e => ApiError::new(e.status_code(), e.to_string()),
        }
    }
}

use super::api::ApiError;

pub use domain::error::sheets::*;

impl From<SheetsError> for ApiError {
    fn from(e: SheetsError) -> Self {
        match e {
            SheetsError::Repo(re) => ApiError::from(re),
            e => ApiError::new(e.status_code(), e.to_string()),
        }
    }
}

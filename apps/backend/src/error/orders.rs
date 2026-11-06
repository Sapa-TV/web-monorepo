use super::api::ApiError;

pub use domain::error::orders::*;

impl From<OrdersServiceError> for ApiError {
    fn from(e: OrdersServiceError) -> Self {
        match e {
            OrdersServiceError::Repo(re) => ApiError::from(re),
            e => ApiError::new(e.status_code(), e.to_string()),
        }
    }
}

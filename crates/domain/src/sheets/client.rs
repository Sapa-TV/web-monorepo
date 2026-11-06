use std::future::Future;

use crate::error::SheetsError;

pub trait SheetsClient: Send + Sync {
    fn get_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
    ) -> impl Future<Output = Result<Vec<Vec<String>>, SheetsError>> + Send;

    fn clear_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
    ) -> impl Future<Output = Result<(), SheetsError>> + Send;

    fn update_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
        values: Vec<Vec<String>>,
    ) -> impl Future<Output = Result<(), SheetsError>> + Send;
}

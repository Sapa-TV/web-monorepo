use std::future::Future;

use crate::error::SheetsError;

pub trait SheetsSettings: Send + Sync {
    fn sheets_spreadsheet_id(&self) -> String;

    fn set_sheets_spreadsheet_id(
        &self,
        spreadsheet_id: &str,
    ) -> impl Future<Output = Result<(), SheetsError>> + Send;
}

use std::future::Future;

use crate::error::Result;

pub trait Transport: Send + Sync {
    fn post_form(
        &self,
        url: &str,
        basic_auth: &str,
        form: &[(&str, &str)],
    ) -> impl Future<Output = Result<String>> + Send;
}

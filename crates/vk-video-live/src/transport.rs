use std::future::Future;

use crate::error::Result;

pub trait Transport: Send + Sync {
    fn post_form(
        &self,
        url: &str,
        basic_auth: &str,
        form: &[(&str, &str)],
    ) -> impl Future<Output = Result<String>> + Send;

    fn get(&self, url: &str, bearer: &str) -> impl Future<Output = Result<String>> + Send;

    fn post_json(
        &self,
        url: &str,
        bearer: &str,
        body: &str,
    ) -> impl Future<Output = Result<String>> + Send;
}

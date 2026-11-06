pub mod client;

pub use domain::sheets::{parse, service};

#[cfg(test)]
#[path = "sheets/service.test.rs"]
mod service_tests;

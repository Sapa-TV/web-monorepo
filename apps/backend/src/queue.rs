pub use domain::queue::{events, service};
pub use model::queue::{entry, repository};

#[cfg(test)]
#[path = "queue/service.test.rs"]
mod service_tests;

#![feature(sync_nonpoison)]
#![feature(nonpoison_mutex)]
#![feature(nonpoison_rwlock)]
#![deny(clippy::exhaustive_structs)]
#![deny(clippy::new_ret_no_self)]

pub mod actions;
pub mod admin;
pub mod config;
pub mod ingress;
pub mod platform;
pub mod presence;
pub mod push;
pub mod runtime;
pub mod sheets;
pub mod srs;
pub mod state;
pub mod stream;

#[cfg(any(test, feature = "test-support"))]
pub mod test_fixtures;

#[cfg(test)]
mod tests;

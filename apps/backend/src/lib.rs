#![feature(sync_nonpoison)]
#![feature(nonpoison_mutex)]
#![feature(nonpoison_rwlock)]
#![deny(clippy::exhaustive_structs)]
#![deny(clippy::new_ret_no_self)]

pub mod admin;
pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod event;
pub mod ingress;
pub mod platform;
pub mod queue;
pub mod random;
pub mod roulette;
pub mod session;
pub mod state;
pub mod stream;
#[cfg(test)]
pub mod test_fixtures;
pub mod user;

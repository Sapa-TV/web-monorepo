#![cfg_attr(test, feature(nonpoison_mutex))]
#![feature(nonpoison_rwlock)]
#![feature(sync_nonpoison)]
#![deny(clippy::exhaustive_structs)]
#![deny(clippy::new_ret_no_self)]
//! Services and business rules. Storage crates are not a dependency on purpose.
//!
//! ```compile_fail
//! use storage_sqlite::SqliteGameOrderRepository;
//! ```

pub mod actions;
pub mod admin;
pub mod config;
pub mod error;
pub mod orders;
pub mod queue;
pub mod random;
pub mod roulette;
pub mod rules;
pub mod session;
pub mod sheets;
pub mod user;

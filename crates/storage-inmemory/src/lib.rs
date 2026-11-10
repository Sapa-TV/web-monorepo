#![feature(sync_nonpoison)]
#![feature(nonpoison_mutex)]
#![deny(clippy::exhaustive_structs)]
#![deny(clippy::new_ret_no_self)]

pub mod inmemory_actions;
pub mod inmemory_admin;
pub mod inmemory_config;
pub mod inmemory_orders;
pub mod inmemory_platform;
pub mod inmemory_platform_credential;
pub mod inmemory_push;
pub mod inmemory_queue;
pub mod inmemory_rarity;
pub mod inmemory_roulette_slots;
pub mod inmemory_rules;
pub mod inmemory_session;
pub mod inmemory_user;

#[cfg(test)]
mod parity;

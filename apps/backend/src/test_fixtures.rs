#![cfg(test)]

use crate::random::StandartRandomProvider;
use crate::state::AppState;

pub fn test_state() -> AppState {
    AppState::new_test_state(StandartRandomProvider)
}

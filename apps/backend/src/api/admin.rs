pub mod twitch;

use crate::state::AppState;

pub fn protected_router() -> axum::Router<AppState> {
    twitch::protected_router()
}

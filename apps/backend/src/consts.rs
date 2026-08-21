//! Process-wide tunables grouped by scope. Rarely change; edit here, not at usage sites.

pub mod actions {
    pub const BUS_CAPACITY: usize = 256;
}

pub mod ingress {
    use std::time::Duration;

    pub const CHANNEL_CAPACITY: usize = 64;
    pub const DEDUP_WINDOW: usize = 1024;
    pub const TWITCH_EVENTSUB_WS_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
    pub const TWITCH_RECONNECT_INITIAL_DELAY: Duration = Duration::from_secs(1);
    pub const TWITCH_RECONNECT_MAX_DELAY: Duration = Duration::from_secs(60);
}

pub mod queue {
    pub const DEFAULT_LIMIT: usize = 20;
    pub const MAX_PAGE_LIMIT: usize = 100;
    pub const RETENTION_SECS: u64 = 24 * 60 * 60;
    pub const CLEANUP_INTERVAL_SECS: u64 = 60 * 60;
}

pub mod roulette {
    pub const TIMEOUT_SECS: u64 = 10;
}

pub mod server {
    pub const PORT: u16 = 3000;
}

pub mod session {
    use std::time::Duration;

    pub const LOGIN_TICKET_TTL: Duration = Duration::from_secs(10 * 60);
    pub const TTL_SECS: u64 = 24 * 60 * 60;
    pub const CLEANUP_INTERVAL_SECS: u64 = 60 * 60;
}

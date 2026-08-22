CREATE TABLE sessions (
    token TEXT PRIMARY KEY NOT NULL,
    twitch_user_id TEXT NOT NULL,
    twitch_user_name TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE TABLE login_tickets (
    ticket TEXT PRIMARY KEY NOT NULL,
    twitch_user_id TEXT NOT NULL,
    twitch_user_name TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

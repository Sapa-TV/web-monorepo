CREATE TABLE admins (
    twitch_id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT,
    is_root INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

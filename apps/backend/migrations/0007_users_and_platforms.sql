CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    display_name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE user_platforms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    platform_id INTEGER NOT NULL REFERENCES platforms(id),
    platform_user_id TEXT NOT NULL,
    platform_username TEXT NOT NULL,
    UNIQUE (platform_id, platform_user_id)
);

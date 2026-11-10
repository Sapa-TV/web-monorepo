INSERT OR IGNORE INTO platforms (id, name) VALUES (4, 'vk');

ALTER TABLE sessions RENAME TO sessions_old;

CREATE TABLE sessions (
    token TEXT PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    platform_id INTEGER REFERENCES platforms(id),
    platform_user_id TEXT,
    platform_username TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

INSERT INTO sessions (
    token, user_id, platform_id, platform_user_id, platform_username, created_at, expires_at
)
SELECT
    token,
    user_id,
    CASE WHEN twitch_user_id IS NULL THEN NULL ELSE 1 END,
    twitch_user_id,
    twitch_user_name,
    created_at,
    expires_at
FROM sessions_old;

DROP TABLE sessions_old;

CREATE INDEX idx_sessions_user_id ON sessions(user_id);

ALTER TABLE login_tickets RENAME TO login_tickets_old;

CREATE TABLE login_tickets (
    ticket TEXT PRIMARY KEY NOT NULL,
    platform_id INTEGER NOT NULL REFERENCES platforms(id),
    platform_user_id TEXT NOT NULL,
    platform_username TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

INSERT INTO login_tickets (
    ticket, platform_id, platform_user_id, platform_username, created_at, expires_at
)
SELECT ticket, 1, twitch_user_id, twitch_user_name, created_at, expires_at
FROM login_tickets_old;

DROP TABLE login_tickets_old;

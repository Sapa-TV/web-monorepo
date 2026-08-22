CREATE TABLE platforms (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

INSERT INTO platforms (id, name) VALUES
    (1, 'twitch'),
    (2, 'youtube'),
    (3, 'vk_video_live');

CREATE TABLE platform_credentials (
    platform_id INTEGER PRIMARY KEY REFERENCES platforms(id),
    credential TEXT NOT NULL
);

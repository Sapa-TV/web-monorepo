CREATE TABLE rarities (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    image TEXT NOT NULL,
    color TEXT NOT NULL
);

INSERT INTO rarities (id, name, display_name, image, color) VALUES
    (1, 'common', 'Common', 'common.png', '#9d9d9d'),
    (2, 'rare', 'Rare', 'rare.png', '#4CAF50'),
    (3, 'epic', 'Epic', 'epic.png', '#9C27B0'),
    (4, 'legendary', 'Legendary', 'legendary.png', '#FFD700');

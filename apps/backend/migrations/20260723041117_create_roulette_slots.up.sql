CREATE TABLE IF NOT EXISTS rarities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    image TEXT NOT NULL,
    color TEXT NOT NULL
);

CREATE TRIGGER IF NOT EXISTS prevent_default_rarity_delete
BEFORE DELETE ON rarities
WHEN OLD.id = 1
BEGIN
    SELECT RAISE(ABORT, 'Cannot delete default rarity');
END;

CREATE TRIGGER IF NOT EXISTS prevent_default_rarity_update
BEFORE UPDATE ON rarities
WHEN OLD.id = 1 AND NEW.id != 1
BEGIN
    SELECT RAISE(ABORT, 'Cannot change default rarity id');
END;

INSERT OR IGNORE INTO rarities (id, name, display_name, image, color) VALUES
    (1, 'common', 'обычный', 'common.png', '#d9e4c6'),
    (2, 'uncommon', 'необычный', 'uncommon.png', '#b0d3e7'),
    (3, 'rare', 'редкий', 'rare.png', '#f1c8ea'),
    (4, 'legendary', 'легендарный', 'legendary.png', '#f4c48b'),
    (5, 'mythical', 'мифический', 'mythical.png', '#733f88');

CREATE TABLE IF NOT EXISTS roulette_slots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    rarity_id INTEGER NOT NULL DEFAULT 1 REFERENCES rarities(id) ON DELETE SET DEFAULT,
    weight INTEGER NOT NULL,
    action TEXT NOT NULL
);

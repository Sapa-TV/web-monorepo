CREATE TABLE roulette_slots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    rarity_id INTEGER NOT NULL REFERENCES rarities(id) ON DELETE CASCADE,
    weight INTEGER NOT NULL,
    action TEXT NOT NULL
);

INSERT INTO roulette_slots (name, rarity_id, weight, action) VALUES
    ('Поболтать', 1, 50, 'chat'),
    ('Подписка', 2, 20, 'subscribe'),
    ('Суперчат', 3, 5, 'superchat'),
    ('Джекпот', 4, 1, 'jackpot');

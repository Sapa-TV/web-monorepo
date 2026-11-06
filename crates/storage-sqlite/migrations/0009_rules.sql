CREATE TABLE rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    trigger_kind TEXT NOT NULL,
    conditions TEXT NOT NULL,
    action_id INTEGER NOT NULL REFERENCES actions(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

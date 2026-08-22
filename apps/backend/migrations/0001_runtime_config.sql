CREATE TABLE runtime_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    payload TEXT NOT NULL
);

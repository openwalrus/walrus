CREATE TABLE IF NOT EXISTS memories (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    metadata TEXT,
    created_at INTEGER NOT NULL,
    accessed_at INTEGER NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 0,
    embedding BLOB
);

CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    key, value, content=memories, content_rowid=rowid
);

CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, key, value)
    VALUES (new.rowid, new.key, new.value);
END;

CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, key, value)
    VALUES ('delete', old.rowid, old.key, old.value);
END;

CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, key, value)
    VALUES ('delete', old.rowid, old.key, old.value);
    INSERT INTO memories_fts(rowid, key, value)
    VALUES (new.rowid, new.key, new.value);
END;

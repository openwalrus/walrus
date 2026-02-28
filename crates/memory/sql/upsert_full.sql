INSERT INTO memories (key, value, metadata, created_at, accessed_at, access_count, embedding)
VALUES (?1, ?2, ?3, ?4, ?4, 0, ?5)
ON CONFLICT(key) DO UPDATE SET
    value = excluded.value,
    metadata = excluded.metadata,
    accessed_at = excluded.accessed_at,
    embedding = excluded.embedding

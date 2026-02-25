INSERT INTO memories (key, value, created_at, accessed_at, access_count)
VALUES (?1, ?2, ?3, ?3, 0)
ON CONFLICT(key) DO UPDATE SET
    value = excluded.value,
    accessed_at = excluded.accessed_at

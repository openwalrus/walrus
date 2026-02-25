SELECT key, value, metadata, created_at, accessed_at, access_count, embedding
FROM memories WHERE key = ?1

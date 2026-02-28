UPDATE memories
SET accessed_at = ?1, access_count = access_count + 1
WHERE key = ?2

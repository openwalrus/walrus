SELECT m.key, m.value, m.metadata, m.created_at, m.accessed_at,
       m.access_count, m.embedding, bm25(memories_fts) AS rank
FROM memories_fts f
JOIN memories m ON m.rowid = f.rowid
WHERE memories_fts MATCH ?1
ORDER BY rank

-- The issue queries a person saved on this Mac, per repository: his words, the query written in the
-- tracker's own language, and the fields a row shows (unit-separator joined). Additive.
CREATE TABLE issue_queries (
    id          TEXT PRIMARY KEY,
    manifest_id TEXT NOT NULL,
    name        TEXT NOT NULL,
    words       TEXT NOT NULL,
    query       TEXT NOT NULL,
    row_fields  TEXT NOT NULL,
    kept_at     TEXT NOT NULL
) STRICT;
CREATE INDEX issue_queries_by_repository ON issue_queries (manifest_id, kept_at);

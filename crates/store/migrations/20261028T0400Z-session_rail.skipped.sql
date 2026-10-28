-- Steps the person skipped: comma-separated indexes, as `nudged` is. Additive; a Session kept before
-- this reads as having skipped none.
ALTER TABLE session_rail ADD COLUMN skipped TEXT NOT NULL DEFAULT '';

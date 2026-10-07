-- The version of its mod a terminal session reported, so Bridge can mark a session whose mod is
-- older than the repository's after a Fleet restart. Absent until reported.
ALTER TABLE sessions ADD COLUMN mod_version TEXT;

-- A hosted Job's running summary, one or two sentences a small model rewrites when a turn ends and
-- when a question goes to the person. One row a session. A table of its own rather than a column on
-- `hosted_sessions`, because `keep_hosting` replaces the whole row and a stale copy would overwrite
-- a newer summary. docs/concepts/session/working-on.md.
CREATE TABLE session_so_far (
    session_id TEXT NOT NULL PRIMARY KEY,
    text       TEXT NOT NULL,
    at         TEXT NOT NULL
) STRICT;

-- When the plan's limit a session hit resets, so Fleet wakes it then. One row a session; deleted when
-- the session takes a turn, a person speaks to it, it is woken or it ends. A table of its own rather
-- than a column on `sessions`, because `keep_session` replaces the whole row. docs/concepts/session.md.
CREATE TABLE session_limit (
    session_id TEXT NOT NULL PRIMARY KEY,
    resets_at  TEXT NOT NULL
) STRICT;

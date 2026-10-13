-- When a session's agent said, through `all_done`, that it has nothing left to work on. One row a
-- session; it is deleted when the person speaks, the agent states a waiting list or a turn starts
-- again. A table of its own rather than a column on `sessions`, because `keep_session` replaces the
-- whole row. docs/concepts/session.md.
CREATE TABLE session_done (
    session_id TEXT NOT NULL PRIMARY KEY,
    since      TEXT NOT NULL
) STRICT;

-- How a session's last turn ended, as Fleet saw it end: when, and the first line of what the agent last said
-- to the person. One row a session; it is deleted when the person speaks or another turn starts. A table of
-- its own rather than a column on `sessions`, because `keep_session` replaces the whole row. Additive.
-- docs/concepts/session.md, *What a Session is doing*.
CREATE TABLE session_turn_end (
    session_id TEXT NOT NULL PRIMARY KEY,
    ended_at   TEXT NOT NULL,
    said       TEXT NOT NULL
) STRICT;

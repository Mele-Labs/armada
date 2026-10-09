-- One `armada check` a Session's agent ran in the slot it held, with its log. A bare `armada check`
-- writes no record of its own, so Fleet keeps these for the thread row and the Checks page.
-- docs/concepts/session.md.
CREATE TABLE session_check_runs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT    NOT NULL,
    manifest_id TEXT    NOT NULL,
    slot        INTEGER NOT NULL,
    name        TEXT    NOT NULL,
    state       TEXT    NOT NULL,
    started_at  TEXT    NOT NULL,
    ended_at    TEXT,
    took_ms     INTEGER,
    log         TEXT    NOT NULL DEFAULT ''
) STRICT;

CREATE INDEX session_check_runs_by_session ON session_check_runs (session_id);

-- A Session's workflow rail and its judges (docs/concepts/session.md, *The rail*, *The judges*).
-- `session_rail` is what the rail has already done for a Session, so it does each once: the step it
-- last stood on (the entry on the timeline is made when it changes), the steps Fleet nudged, and the
-- steps a boundary judge already read. `session_concerns` are the concerns a judge raised, each kept
-- until the person sends it or dismisses it and then kept settled. `session_judge_reading` is the
-- outcome of the last reading, so zero concerns can be told from no reading. No foreign keys, as
-- `session_build` has none. Additive.
CREATE TABLE session_rail (
    session_id TEXT NOT NULL PRIMARY KEY,
    step       INTEGER,
    nudged     TEXT NOT NULL DEFAULT '',
    judged     TEXT NOT NULL DEFAULT ''
) STRICT;

CREATE TABLE session_concerns (
    session_id TEXT NOT NULL,
    n          INTEGER NOT NULL,
    text       TEXT NOT NULL,
    since      TEXT NOT NULL,
    from_step  INTEGER,
    from_label TEXT NOT NULL DEFAULT '',
    cites      TEXT NOT NULL DEFAULT '',
    how        TEXT CHECK (how IN ('sent', 'dismissed')),
    settled_at TEXT,
    PRIMARY KEY (session_id, n)
) STRICT;

CREATE TABLE session_judge_reading (
    session_id TEXT NOT NULL PRIMARY KEY,
    at         TEXT NOT NULL,
    concerns   INTEGER NOT NULL
) STRICT;

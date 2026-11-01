-- A Job's Judge readings, one a (kind, artifact): asking again for the same artifact replaces its
-- reading and its notes, and the person's `read` goes with it. A note's `about` is the part in the
-- artifact's own terms, `place` the same part as a person reads it, `cites` one per line, and `how`
-- and `settled_at` NULL while it waits. Additive; the adversarial tables and the Session judge's
-- concerns are left where they were and nothing reads them. docs/concepts/judge.md.
CREATE TABLE judge_readings (
    session_id  TEXT    NOT NULL,
    id          TEXT    NOT NULL,
    kind        TEXT    NOT NULL,
    artifact_id TEXT    NOT NULL,
    title       TEXT    NOT NULL,
    prompt      TEXT    NOT NULL DEFAULT '',
    state       TEXT    NOT NULL,
    asked_at    TEXT    NOT NULL,
    finished_at TEXT,
    failed      TEXT,
    read        INTEGER NOT NULL DEFAULT 0,
    cost_micros INTEGER,
    PRIMARY KEY (session_id, kind, artifact_id)
) STRICT;

CREATE TABLE judge_notes (
    session_id  TEXT    NOT NULL,
    reading     TEXT    NOT NULL,
    position    INTEGER NOT NULL,
    id          TEXT    NOT NULL,
    about       TEXT,
    claim       TEXT    NOT NULL,
    place       TEXT,
    expected    TEXT,
    found       TEXT,
    impact      TEXT,
    cites       TEXT    NOT NULL DEFAULT '',
    how         TEXT,
    settled_at  TEXT,
    PRIMARY KEY (session_id, reading, id)
) STRICT;

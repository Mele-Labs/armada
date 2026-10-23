-- Watches: a command Fleet runs on a cadence for one Job, its last 50 readings, the parts the agent
-- set aside, its verdicts, and the notices told of it, so a restart carries on and tells nothing
-- twice. `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`. Additive.
CREATE TABLE watches (
    id                     TEXT PRIMARY KEY,
    session_id             TEXT NOT NULL,
    name                   TEXT NOT NULL,
    command                TEXT NOT NULL,
    recipe                 TEXT,
    environment            TEXT,
    every_seconds          INTEGER NOT NULL,
    gives_up_after_seconds INTEGER NOT NULL,
    follows                INTEGER,
    state                  TEXT NOT NULL,
    live                   INTEGER NOT NULL,
    started_at             TEXT NOT NULL,
    reported               TEXT NOT NULL DEFAULT '[]',
    changed_at             TEXT NOT NULL,
    ended_at               TEXT,
    unsaid                 TEXT
) STRICT;
CREATE INDEX watches_by_session ON watches (session_id);
CREATE TABLE watch_readings (
    watch_id TEXT NOT NULL,
    n        INTEGER NOT NULL,
    at       TEXT NOT NULL,
    kind     TEXT NOT NULL,
    state    TEXT,
    finished INTEGER,
    says     TEXT,
    link     TEXT,
    parts    TEXT,
    why      TEXT,
    output   TEXT,
    PRIMARY KEY (watch_id, n)
) STRICT;
CREATE TABLE watch_set_aside (
    watch_id TEXT NOT NULL,
    part     TEXT NOT NULL,
    sentence TEXT NOT NULL,
    at       TEXT NOT NULL,
    PRIMARY KEY (watch_id, part)
) STRICT;
CREATE TABLE watch_verdicts (
    watch_id TEXT NOT NULL,
    n        INTEGER NOT NULL,
    at       TEXT NOT NULL,
    verdict  TEXT NOT NULL,
    sentence TEXT,
    parts    TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY (watch_id, n)
) STRICT;
CREATE TABLE watch_notices (
    watch_id TEXT NOT NULL,
    said     TEXT NOT NULL,
    told_at  TEXT NOT NULL,
    PRIMARY KEY (watch_id, said)
) STRICT;

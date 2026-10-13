-- What happened to a Job, a Session, a pull request, a slot or main, one row each, with the instant it
-- happened. Written as Fleet sees each transition; read as a window by `get_fleet_timeline`. No foreign
-- key: a forgotten Job's history stays, as `check_runs` does. `seq` never repeats after a prune. Additive.
CREATE TABLE timeline_events (
    seq          INTEGER PRIMARY KEY AUTOINCREMENT,
    at           TEXT NOT NULL,
    kind         TEXT NOT NULL,
    subject_kind TEXT NOT NULL CHECK (subject_kind IN ('job', 'session', 'pull_request', 'main')),
    subject_id   TEXT NOT NULL,
    subject_url  TEXT,
    title        TEXT,
    repo         TEXT,
    tone         TEXT NOT NULL,
    says         TEXT NOT NULL,
    detail       TEXT,
    live         INTEGER NOT NULL CHECK (live IN (0, 1)),
    over         INTEGER NOT NULL CHECK (over IN (0, 1))
) STRICT;
CREATE INDEX timeline_events_by_time ON timeline_events (at, seq);

-- Who took a Job over, the mark Fleet set for the handoff, the Drone's narrative and how the pilot ended. Additive.
CREATE TABLE job_pilots (
    job_id     TEXT NOT NULL PRIMARY KEY,
    reason     TEXT NOT NULL CHECK (reason IN ('take_over', 'restart_step', 'assist')),
    session_id TEXT,
    marked_run TEXT NOT NULL,
    marked_at  TEXT NOT NULL,
    piloted_at TEXT,
    trying_to  TEXT,
    blocked_by TEXT,
    tried      TEXT,
    exit_how   TEXT CHECK (exit_how IN ('submitted', 'attested', 'superseded')),
    ended_at   TEXT,
    note       TEXT
) STRICT;

-- When a Job's agent stopped on its own, and when Fleet was down, so the split of a Job's time can
-- tell a process that died from one reaped after a quiet spell (docs/concepts/session.md).
-- `session_restarts` kept a death only while a turn ran, and only the process coming back after a
-- Fleet restart, never the Fleet being away. A reaping is Fleet's own act and is not a stop.
-- Additive.
CREATE TABLE session_stops (
    session_id TEXT NOT NULL,
    at         TEXT NOT NULL,
    PRIMARY KEY (session_id, at)
) STRICT, WITHOUT ROWID;

-- One row, written every few seconds while Fleet runs: when it was last seen up. The next start
-- reads it to say how long it was away.
CREATE TABLE fleet_alive (
    only_row INTEGER NOT NULL PRIMARY KEY CHECK (only_row = 1),
    at       TEXT NOT NULL
) STRICT;

-- A stretch Fleet was down: from the last time it was seen up to the start that found it so.
CREATE TABLE fleet_outages (
    began_at TEXT NOT NULL PRIMARY KEY,
    ended_at TEXT NOT NULL
) STRICT, WITHOUT ROWID;

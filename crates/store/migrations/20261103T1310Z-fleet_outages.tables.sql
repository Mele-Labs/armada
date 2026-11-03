-- One row, written every few seconds while Fleet runs: when it was last seen up. The next start
-- reads it to say how long it was away, so a Job's stop can say Fleet was down. Additive. `IF NOT
-- EXISTS` since a preview store may already hold these. docs/concepts/session/what-a-job-is-doing.md.
CREATE TABLE IF NOT EXISTS fleet_alive (
    only_row INTEGER NOT NULL PRIMARY KEY CHECK (only_row = 1),
    at       TEXT NOT NULL
) STRICT;

-- A stretch Fleet was down: from the last time it was seen up to the start that found it so.
CREATE TABLE IF NOT EXISTS fleet_outages (
    began_at TEXT NOT NULL PRIMARY KEY,
    ended_at TEXT NOT NULL
) STRICT, WITHOUT ROWID;

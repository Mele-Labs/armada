-- The stop each Job was told about and the stop passed over, so a stop draws one nudge across a
-- restart and a stop the person made is never read as one. A stop is named by `sessions.last_turn_at`.
-- Every live Job's current stop is passed over here, so landing this raises no flood of nudges.
-- docs/concepts/session/what-a-job-is-doing.md.
CREATE TABLE session_stops (
    session_id  TEXT PRIMARY KEY,
    told_stop   TEXT,
    told_at     TEXT,
    unheard     INTEGER NOT NULL DEFAULT 0,
    passed_over TEXT
) STRICT;

INSERT INTO session_stops (session_id, passed_over)
    SELECT id, last_turn_at FROM sessions WHERE state = 'live' AND last_turn_at IS NOT NULL;

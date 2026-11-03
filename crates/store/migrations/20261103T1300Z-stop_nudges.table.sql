-- The stop each Job was told about and the stop passed over, so a stop draws one nudge across a
-- restart and a stop the person made is never read as one. A stop is named by `sessions.last_turn_at`.
-- `repair_job` is the Job started to bring a dead one back, `repair_held` that an unhealthy Fleet
-- started none, and `repairs`, on a repair Job's own row, the Job it repairs.
-- Every live Job's current stop is passed over here, so landing this raises no flood of nudges.
-- docs/concepts/session/what-a-job-is-doing.md.
CREATE TABLE stop_nudges (
    session_id  TEXT PRIMARY KEY,
    told_stop   TEXT,
    told_at     TEXT,
    unheard     INTEGER NOT NULL DEFAULT 0,
    passed_over TEXT,
    repair_job  TEXT,
    repair_held INTEGER NOT NULL DEFAULT 0,
    repairs     TEXT
) STRICT;

INSERT INTO stop_nudges (session_id, passed_over)
    SELECT id, last_turn_at FROM sessions WHERE state = 'live' AND last_turn_at IS NOT NULL;

-- Each fact that woke a Job's Commander, once. The key is the problem and the run of it the wake
-- was for, so a tick that sees the same stall again, or a Fleet that restarts, finds the row and
-- wakes nothing. `standing` is the Job's status, steps and task counts at the wake, and `run`
-- counts the wakes in a row on one (problem, step) that found `standing` unchanged: the loop
-- bound. For a cap, `step` is empty and `attempt` is the cap in force, so a raised cap re-arms.
CREATE TABLE job_commander_wakes (
    seq      INTEGER PRIMARY KEY,
    job_id   TEXT    NOT NULL REFERENCES jobs(job_id),
    problem  TEXT    NOT NULL,
    step     TEXT    NOT NULL,
    attempt  INTEGER NOT NULL,
    standing TEXT    NOT NULL,
    run      INTEGER NOT NULL,
    UNIQUE (job_id, problem, step, attempt)
) STRICT;

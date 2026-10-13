-- A repair Drone a Job's Commander sent that has not come back. The hold on the Job lives here and
-- not only in Fleet's memory, so a Fleet that restarts with one out finds the row, knows the Drone
-- went with the process that read it, and wakes the Commander instead of leaving the Job held with
-- nothing running. One row per Job, written when the repair is sent and removed when it has come
-- back or been dealt with. `broke` is the word the Job's log carries and `drone_id` names the Drone.
CREATE TABLE job_commander_repairs (
    job_id   TEXT PRIMARY KEY REFERENCES jobs(job_id),
    step     TEXT NOT NULL,
    broke    TEXT NOT NULL,
    drone_id TEXT NOT NULL
) STRICT;

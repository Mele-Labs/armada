-- A Job's Commander is a fourth signer. Which moves of a Job were its acts: beside job_event_via,
-- whose CHECK cannot be widened without a rebuild; a seq here is read as via = 'commander'.
CREATE TABLE job_event_commander (
    seq    INTEGER PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(job_id)
) STRICT;

-- Who made a plan change when it was the Job's Commander. by_step and by_attempt name a step's run
-- and are both null for a person and for a Commander, so this column is the only thing that
-- tells the two apart.
ALTER TABLE job_work_plan_changes ADD COLUMN by_commander INTEGER NOT NULL DEFAULT 0
    CHECK (by_commander IN (0, 1));

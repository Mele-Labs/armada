-- A step's entry baseline. Nothing is backfilled: a step that began before
-- this reads as one Fleet never saw start, which fails closed.
CREATE TABLE job_step_baselines (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    entries TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id)
) STRICT;

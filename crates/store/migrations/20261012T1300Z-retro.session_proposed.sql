-- The Session started for an agreed retro item, beside the Job that agreeing
-- may propose. Null on every item kept before and nothing is backfilled; at
-- most one of `job_proposed` and `session_proposed` is ever set, which the
-- writes keep (a Session is only recorded where no Job is).
ALTER TABLE job_retro_items ADD COLUMN session_proposed TEXT;
ALTER TABLE session_retro_items ADD COLUMN session_proposed TEXT;

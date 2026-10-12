-- The Session started for an agreed retro item, beside the Job that agreeing
-- may propose. Null on every item kept before and nothing is backfilled; at
-- most one of `job_proposed` and `session_proposed` is ever set, which the
-- writes keep (a Session is only recorded where no Job is).
ALTER TABLE job_retro_items ADD COLUMN session_proposed TEXT;
ALTER TABLE session_retro_items ADD COLUMN session_proposed TEXT;

-- Why the brief was not sent to that Session, where the send was refused.
-- Null when it was sent. Kept so a second press can say so again.
ALTER TABLE job_retro_items ADD COLUMN session_send_failed TEXT;
ALTER TABLE session_retro_items ADD COLUMN session_send_failed TEXT;

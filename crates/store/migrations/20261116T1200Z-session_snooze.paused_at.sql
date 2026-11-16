-- The pause marker (`hosted_sessions.paused_at`) the snooze set when it paused the Job, NULL when
-- it did not. When the snooze ends, Fleet resumes the Job only while its marker is still this one:
-- a Job paused by the person before the snooze, or resumed and paused again meanwhile, stays as
-- he left it. docs/concepts/session/snoozing-a-job.md.
ALTER TABLE session_snooze ADD COLUMN paused_at TEXT;

-- A Session can take a red as a Job does. `job_id` holds the Session's id where `holder_kind` is
-- 'session'; `forked_from` names the Session a fork was made from for the red. Additive: a row kept
-- before this reads as a Job's.
ALTER TABLE main_ci_fixes ADD COLUMN holder_kind TEXT NOT NULL DEFAULT 'job' CHECK (holder_kind IN ('job', 'session'));
ALTER TABLE main_ci_fixes ADD COLUMN forked_from TEXT;

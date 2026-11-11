-- The project of his a Job was started for, by id (`StartSession.project`), so the Job keeps it
-- across a restart. Additive: NULL on every row that was started for none.
ALTER TABLE hosted_sessions ADD COLUMN project TEXT;

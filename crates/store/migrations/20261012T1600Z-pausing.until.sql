-- When a Job paused by Fleet for the plan's limit may be started again. Null for every other pause.
-- Admission leaves the Job alone until then; `fleet::limit_hold`.
ALTER TABLE jobs ADD COLUMN pause_until TEXT;

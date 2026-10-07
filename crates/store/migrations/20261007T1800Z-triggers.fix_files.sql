-- The files a failed Trigger's held fix changes, one path to a line, so Bridge
-- can list them beside the owner's choice. docs/concepts/trigger.md. Additive.
ALTER TABLE job_triggers ADD COLUMN fix_files TEXT NOT NULL DEFAULT '';

-- A written task held by a Job: `held_by` is the Job's id and `asked_at` is when the done? ask was
-- raised on it. `state` gains `held` and `asked` (unchecked text, as it was). Both are empty for a
-- task no Job holds, and an issue row never has either. Additive.
ALTER TABLE project_rows ADD COLUMN held_by TEXT NOT NULL DEFAULT '';
ALTER TABLE project_rows ADD COLUMN asked_at TEXT NOT NULL DEFAULT '';

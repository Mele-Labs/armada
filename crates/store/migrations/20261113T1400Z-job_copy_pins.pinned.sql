-- Whether a Job's copy of its workflow is a pin a publish made (1) or the Job's own edit (0, and on every copy kept
-- before this). A publish pins each Job it would move to the text that Job was running, so the person chooses which
-- Jobs move to the new version; a Job's own step edit keeps its copy with 0.
-- Additive.
ALTER TABLE job_copy_workflows ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;

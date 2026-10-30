-- When ending the Job ended the instance, empty for one that is live or that reached an end state
-- of its own. Ending a Job keeps each instance still in flight as ended, standing where it was, so
-- the Job's Workflow view draws it so and no reader moves it again.
ALTER TABLE lifecycle_instances ADD COLUMN ended_at TEXT NOT NULL DEFAULT '';
-- When the agent was refused `all_done` or `finish_up` for the instances the Job holds, empty
-- until it is. It is what says the agent was told to wait, so that the last instance reaching an
-- end state is told to that agent once and to no other. Additive: both columns default to empty,
-- and a Job kept before this reads as live and never refused.
ALTER TABLE lifecycle_jobs ADD COLUMN held_at TEXT NOT NULL DEFAULT '';

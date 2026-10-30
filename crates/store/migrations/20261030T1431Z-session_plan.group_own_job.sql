-- Whether a plan's group goes to a Job of its own. 1 is yes. A group kept before this reads 0.
-- Additive: the column defaults to 0.
ALTER TABLE session_plan_groups ADD COLUMN own_job INTEGER NOT NULL DEFAULT 0;

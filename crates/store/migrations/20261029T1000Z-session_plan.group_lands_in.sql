-- The short label of the pull request a plan's group lands in (`PR 1`). Empty for a group kept before
-- this, which reads as landing in `PR 1`. Additive.
ALTER TABLE session_plan_groups ADD COLUMN lands_in TEXT;

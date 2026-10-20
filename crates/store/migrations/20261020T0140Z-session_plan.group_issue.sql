-- The issue a plan group names (docs/concepts/session/the-plan.md). Null where the group names none,
-- and for every group recorded before this column. Additive.
ALTER TABLE session_plan_groups ADD COLUMN issue INTEGER;

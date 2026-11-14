-- The titles of the sketches a plan names, one a line: the plan's own, and each group's. A plan kept
-- before this names none. Additive: both columns default to empty.
ALTER TABLE session_plan ADD COLUMN sketches TEXT NOT NULL DEFAULT '';
ALTER TABLE session_plan_groups ADD COLUMN sketches TEXT NOT NULL DEFAULT '';

-- What the person reads first of a plan: a few plain lines on the plan (`summary`), and one line a group
-- gives him (`gives`). Both empty for a plan recorded before this, which reads as having none. Additive.
ALTER TABLE session_plan ADD COLUMN summary TEXT NOT NULL DEFAULT '';
ALTER TABLE session_plan_groups ADD COLUMN gives TEXT;

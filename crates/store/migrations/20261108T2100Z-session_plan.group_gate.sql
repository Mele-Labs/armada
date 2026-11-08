-- The fact that ends a plan group, as the agent named it: `plan_approved`, `walk_approved`,
-- `sketch_approved`, `document_approved`, `checks_green` or `merged`. NULL is no gate.
ALTER TABLE session_plan_groups ADD COLUMN gate TEXT;

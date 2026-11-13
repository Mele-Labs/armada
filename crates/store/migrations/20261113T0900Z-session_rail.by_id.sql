-- Which steps a Session's rail kept, by the step's id and not its index in the workflow, so a workflow save that
-- inserts, drops or reorders steps does not move a running Job. `by_id` is 0 for a row kept before this: its numbers
-- are indexes and are read as such until Fleet rewrites it. A row kept by id holds the standing step in `step_id`
-- (`step` is an INTEGER in a STRICT table and cannot hold text) and the lists as JSON arrays of ids, since a step id is
-- free text. A build from before this that writes such a row back leaves `by_id` at 1 over numbers it wrote. Additive.
ALTER TABLE session_rail ADD COLUMN by_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_rail ADD COLUMN step_id TEXT;

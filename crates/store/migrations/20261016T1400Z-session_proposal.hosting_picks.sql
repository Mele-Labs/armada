-- What the proposer settled for a hosted session at dispatch and what the person has since changed.
-- `workflow` is the workflow id the session runs under (NULL is a custom session). `proposed` lists,
-- comma separated, the fields the proposer picked and nobody has changed since. `turn_model` and
-- `turn_effort` are what the last turn started on, so the next turn can tell the agent a change.
-- Additive: every column is NULL or defaults to the empty list.
ALTER TABLE hosted_sessions ADD COLUMN workflow TEXT;
ALTER TABLE hosted_sessions ADD COLUMN proposed TEXT NOT NULL DEFAULT '';
ALTER TABLE hosted_sessions ADD COLUMN turn_model TEXT;
ALTER TABLE hosted_sessions ADD COLUMN turn_effort TEXT;

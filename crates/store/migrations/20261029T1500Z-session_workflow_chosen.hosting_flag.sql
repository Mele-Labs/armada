-- The person picked the workflow on the start card, None included, so a default does not go over
-- it after a restart. 1 is chosen. Additive: the column defaults to 0.
ALTER TABLE hosted_sessions ADD COLUMN workflow_chosen INTEGER NOT NULL DEFAULT 0;

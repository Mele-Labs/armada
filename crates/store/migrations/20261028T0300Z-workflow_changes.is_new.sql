-- A change that adds a workflow nothing held before, ready from the start with no earlier text.
-- Additive; a change kept before this reads as a change to an existing definition.
ALTER TABLE workflow_changes ADD COLUMN is_new INTEGER NOT NULL DEFAULT 0;

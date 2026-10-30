-- A change a Job proposed to a workflow that already exists, ready from the start with the text it
-- replaces and the text proposed. Additive; a change kept before this was asked for or drafted.
ALTER TABLE workflow_changes ADD COLUMN from_job INTEGER NOT NULL DEFAULT 0;

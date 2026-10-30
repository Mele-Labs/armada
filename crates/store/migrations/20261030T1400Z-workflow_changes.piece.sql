-- A change a Job proposed to a rule or a lifecycle for Kit, kept in the row a workflow's is, so one approval
-- holds both. Additive; empty on every change kept before this, which was to a workflow.
ALTER TABLE workflow_changes ADD COLUMN piece TEXT NOT NULL DEFAULT '';

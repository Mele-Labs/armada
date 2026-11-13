-- The project a change to a workflow is for, where `source` is `project`; empty otherwise and on every change kept
-- before this. And whether the drafted text landed in the owner's draft of the workflow (1) or only waits on Approve (0).
-- Additive.
ALTER TABLE workflow_changes ADD COLUMN project TEXT NOT NULL DEFAULT '';
ALTER TABLE workflow_changes ADD COLUMN drafted INTEGER NOT NULL DEFAULT 0;

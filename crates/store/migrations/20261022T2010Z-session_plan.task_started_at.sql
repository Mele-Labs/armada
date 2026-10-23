-- When a task last began to be in progress, by the agent's own task list. Null for a task not in progress,
-- and for one that was before this column: it is then older than any that has a time. Additive.
ALTER TABLE session_plan_tasks ADD COLUMN started_at TEXT;

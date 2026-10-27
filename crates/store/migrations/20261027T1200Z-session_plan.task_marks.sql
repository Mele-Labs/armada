-- What Fleet keeps to say a task in progress is stale, and the marks it set itself. `calls` is how many
-- tool calls the Session's own agent has made since its plan was first recorded. A task's `touched_at` and
-- `touched_call` are when its mark last moved or Fleet last saw work under its group's paths; `marked` is
-- the state Fleet set from proof (`in_progress` or `completed`) and `marked_why` that proof; `told` is
-- whether the agent was told of the task, which happens once. A plan kept before this reads with no count,
-- no touch and no mark. Additive.
ALTER TABLE session_plan ADD COLUMN calls INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_plan_tasks ADD COLUMN touched_at TEXT;
ALTER TABLE session_plan_tasks ADD COLUMN touched_call INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_plan_tasks ADD COLUMN marked TEXT;
ALTER TABLE session_plan_tasks ADD COLUMN marked_why TEXT;
ALTER TABLE session_plan_tasks ADD COLUMN told INTEGER NOT NULL DEFAULT 0;

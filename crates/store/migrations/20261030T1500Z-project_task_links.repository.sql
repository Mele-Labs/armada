-- The repository each issue and pull request was linked to its task in, which moving the task to another
-- repository keeps. Additive, so not backfilled: an empty one is a link kept before this, or by a build
-- without it, and is its task's repository, which the store stamps on it before the task first moves.
ALTER TABLE project_task_issues ADD COLUMN manifest_id TEXT NOT NULL DEFAULT '';
ALTER TABLE project_task_pulls ADD COLUMN manifest_id TEXT NOT NULL DEFAULT '';

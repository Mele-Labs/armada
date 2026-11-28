-- A sub project's goal, its orchestrator Job and how its Goal met? card was answered. Additive.
-- docs/concepts/feature-catalog.md, *Sub projects*.
--
-- `goal` is his words; a sub project kept before goals has '' and is read with its name. `orchestrator` is the session id of the Job that
-- coordinates it, NULL until one is started. `orchestrator_asked_at` is when Fleet first tried to start one for a sub
-- project kept without, so it tries once. `not_yet_at` is his last Not yet; `ended_at` his Agree, which folds it away.
ALTER TABLE sub_projects ADD COLUMN goal TEXT NOT NULL DEFAULT '';
ALTER TABLE sub_projects ADD COLUMN orchestrator TEXT;
ALTER TABLE sub_projects ADD COLUMN orchestrator_asked_at TEXT;
ALTER TABLE sub_projects ADD COLUMN not_yet_at TEXT;
ALTER TABLE sub_projects ADD COLUMN ended_at TEXT;

-- An added Script step's repair, on the step's own row: what the repair has
-- come to, as a Trigger's firing carries it. docs/concepts/trigger.md. Additive
-- on purpose: `state` keeps its CHECK, so a repair in flight is `repair_state`
-- and the row reads `running` to a build that does not know it.
ALTER TABLE job_additions ADD COLUMN repair_state TEXT
    CHECK (repair_state IN ('repairing', 'rerunning', 'fix_ready'));
ALTER TABLE job_additions ADD COLUMN repair_tries INTEGER NOT NULL DEFAULT 0;
ALTER TABLE job_additions ADD COLUMN repair_branch TEXT;
ALTER TABLE job_additions ADD COLUMN fix_choice TEXT
    CHECK (fix_choice IN ('this_branch', 'new_pr'));
ALTER TABLE job_additions ADD COLUMN fix_pr TEXT;
ALTER TABLE job_additions ADD COLUMN repair_settled_at TEXT;
ALTER TABLE job_additions ADD COLUMN fix_files TEXT NOT NULL DEFAULT '';

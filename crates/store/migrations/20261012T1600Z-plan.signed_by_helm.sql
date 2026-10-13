-- Who made a plan change when it was a Helm session acting for the person. by_step and by_attempt
-- are null for a person, for Helm and for a Commander, so this column and by_commander are the only
-- things that tell the three apart; a change is at most one of them.
ALTER TABLE job_work_plan_changes ADD COLUMN by_helm INTEGER NOT NULL DEFAULT 0
    CHECK (by_helm IN (0, 1) AND by_helm + by_commander <= 1);

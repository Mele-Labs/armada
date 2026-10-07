-- breaking
-- A failed Trigger's repair: three more states, and what the repair has come
-- to on the firing's own row. docs/concepts/trigger.md. SQLite cannot widen a
-- CHECK, so the table is rebuilt with its rows.
CREATE TABLE job_triggers_repairable (
    firing_id      INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id         TEXT NOT NULL REFERENCES jobs(job_id),
    name           TEXT NOT NULL,
    moment         TEXT NOT NULL CHECK (moment IN ('step_starts', 'step_passes', 'pr_opened')),
    step_id        TEXT NOT NULL,
    source         TEXT NOT NULL CHECK (source IN ('armada', 'repository', 'machine')),
    state          TEXT NOT NULL CHECK (state IN ('skipped', 'running', 'passed', 'failed', 'awaiting_owner', 'repairing', 'rerunning', 'fix_ready')),
    skipped_why    TEXT CHECK (skipped_why IN ('not_in_this_repo', 'skill_not_run')),
    skipped_name   TEXT,
    exit_code      INTEGER,
    block_on_fail  INTEGER NOT NULL CHECK (block_on_fail IN (0, 1)),
    repair_on_fail INTEGER NOT NULL CHECK (repair_on_fail IN (0, 1)),
    started_at     TEXT NOT NULL,
    ended_at       TEXT,
    repair_tries   INTEGER NOT NULL DEFAULT 0,
    repair_branch  TEXT,
    fix_choice     TEXT CHECK (fix_choice IN ('this_branch', 'new_pr')),
    fix_pr         TEXT
) STRICT;
INSERT INTO job_triggers_repairable
    (firing_id, job_id, name, moment, step_id, source, state, skipped_why, skipped_name,
     exit_code, block_on_fail, repair_on_fail, started_at, ended_at)
SELECT firing_id, job_id, name, moment, step_id, source, state, skipped_why, skipped_name,
     exit_code, block_on_fail, repair_on_fail, started_at, ended_at
FROM job_triggers;
DROP TABLE job_triggers;
ALTER TABLE job_triggers_repairable RENAME TO job_triggers;
CREATE INDEX job_triggers_by_job ON job_triggers (job_id, firing_id);

-- breaking
-- A Trigger or an added step that blocks and fails holds its Job: a `held`
-- state, a skip by the owner as a reason, and `released`, which says a hold on
-- this firing was let go and the next entry to its moment has not yet passed
-- it. docs/concepts/trigger.md. SQLite cannot widen a CHECK, so both tables
-- are rebuilt with their rows.
CREATE TABLE job_triggers_blocking (
    firing_id      INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id         TEXT NOT NULL REFERENCES jobs(job_id),
    name           TEXT NOT NULL,
    moment         TEXT NOT NULL CHECK (moment IN ('step_starts', 'step_passes', 'pr_opened')),
    step_id        TEXT NOT NULL,
    source         TEXT NOT NULL CHECK (source IN ('armada', 'repository', 'machine')),
    state          TEXT NOT NULL CHECK (state IN ('skipped', 'running', 'passed', 'failed', 'awaiting_owner', 'repairing', 'rerunning', 'fix_ready', 'held')),
    skipped_why    TEXT CHECK (skipped_why IN ('not_in_this_repo', 'skill_not_run', 'by_owner')),
    skipped_name   TEXT,
    exit_code      INTEGER,
    block_on_fail  INTEGER NOT NULL CHECK (block_on_fail IN (0, 1)),
    repair_on_fail INTEGER NOT NULL CHECK (repair_on_fail IN (0, 1)),
    started_at     TEXT NOT NULL,
    ended_at       TEXT,
    repair_tries   INTEGER NOT NULL DEFAULT 0,
    repair_branch  TEXT,
    fix_choice     TEXT CHECK (fix_choice IN ('this_branch', 'new_pr')),
    fix_pr         TEXT,
    repair_settled_at TEXT,
    fix_files      TEXT NOT NULL DEFAULT '',
    released       INTEGER NOT NULL DEFAULT 0 CHECK (released IN (0, 1))
) STRICT;
INSERT INTO job_triggers_blocking
    (firing_id, job_id, name, moment, step_id, source, state, skipped_why, skipped_name,
     exit_code, block_on_fail, repair_on_fail, started_at, ended_at, repair_tries,
     repair_branch, fix_choice, fix_pr, repair_settled_at, fix_files)
SELECT firing_id, job_id, name, moment, step_id, source, state, skipped_why, skipped_name,
     exit_code, block_on_fail, repair_on_fail, started_at, ended_at, repair_tries,
     repair_branch, fix_choice, fix_pr, repair_settled_at, fix_files
FROM job_triggers;
DROP TABLE job_triggers;
ALTER TABLE job_triggers_blocking RENAME TO job_triggers;
CREATE INDEX job_triggers_by_job ON job_triggers (job_id, firing_id);

CREATE TABLE job_additions_blocking (
    job_id         TEXT NOT NULL REFERENCES jobs(job_id),
    addition_id    TEXT NOT NULL,
    ordinal        INTEGER NOT NULL,
    kind           TEXT NOT NULL CHECK (kind IN ('script', 'skill', 'drone')),
    runs_text      TEXT NOT NULL,
    moment         TEXT NOT NULL CHECK (moment IN ('step_starts', 'step_passes', 'pr_opened')),
    step_id        TEXT NOT NULL,
    block_on_fail  INTEGER NOT NULL CHECK (block_on_fail IN (0, 1)),
    repair_on_fail INTEGER NOT NULL CHECK (repair_on_fail IN (0, 1)),
    placed         TEXT NOT NULL CHECK (placed IN ('approval', 'running')),
    added_at       TEXT NOT NULL,
    state          TEXT CHECK (state IN ('skipped', 'running', 'passed', 'failed', 'awaiting_owner', 'held')),
    not_run_why    TEXT CHECK (not_run_why IN ('not_in_this_repo', 'skill_not_run', 'drone_step_not_run', 'by_owner')),
    not_run_name   TEXT,
    exit_code      INTEGER,
    started_at     TEXT,
    ended_at       TEXT,
    kept           TEXT CHECK (kept IN ('repository', 'machine')),
    removed_at     TEXT,
    released       INTEGER NOT NULL DEFAULT 0 CHECK (released IN (0, 1)),
    PRIMARY KEY (job_id, addition_id)
) STRICT;
INSERT INTO job_additions_blocking
    (job_id, addition_id, ordinal, kind, runs_text, moment, step_id, block_on_fail,
     repair_on_fail, placed, added_at, state, not_run_why, not_run_name, exit_code,
     started_at, ended_at, kept, removed_at)
SELECT job_id, addition_id, ordinal, kind, runs_text, moment, step_id, block_on_fail,
     repair_on_fail, placed, added_at, state, not_run_why, not_run_name, exit_code,
     started_at, ended_at, kept, removed_at
FROM job_additions;
DROP TABLE job_additions;
ALTER TABLE job_additions_blocking RENAME TO job_additions;
CREATE INDEX job_additions_by_job ON job_additions (job_id, ordinal);

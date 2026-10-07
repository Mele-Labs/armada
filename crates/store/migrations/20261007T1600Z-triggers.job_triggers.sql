-- The Triggers frozen onto a Job at its approval, one row per step each fires
-- on, and every firing of one. docs/concepts/trigger.md. Additive.
CREATE TABLE job_frozen_triggers (
    job_id        TEXT NOT NULL REFERENCES jobs(job_id),
    ordinal       INTEGER NOT NULL,
    name          TEXT NOT NULL,
    moment        TEXT NOT NULL CHECK (moment IN ('step_starts', 'step_passes', 'pr_opened')),
    step_id       TEXT NOT NULL,
    source        TEXT NOT NULL CHECK (source IN ('armada', 'repository', 'machine')),
    resolution    TEXT NOT NULL CHECK (resolution IN ('command', 'skill', 'not_in_this_repo')),
    runs_name     TEXT NOT NULL,
    asks_first    INTEGER NOT NULL CHECK (asks_first IN (0, 1)),
    block_on_fail INTEGER NOT NULL CHECK (block_on_fail IN (0, 1)),
    repair_on_fail INTEGER NOT NULL CHECK (repair_on_fail IN (0, 1)),
    PRIMARY KEY (job_id, ordinal)
) STRICT;

CREATE TABLE job_triggers (
    firing_id      INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id         TEXT NOT NULL REFERENCES jobs(job_id),
    name           TEXT NOT NULL,
    moment         TEXT NOT NULL CHECK (moment IN ('step_starts', 'step_passes', 'pr_opened')),
    step_id        TEXT NOT NULL,
    source         TEXT NOT NULL CHECK (source IN ('armada', 'repository', 'machine')),
    state          TEXT NOT NULL CHECK (state IN ('skipped', 'running', 'passed', 'failed', 'awaiting_owner')),
    skipped_why    TEXT CHECK (skipped_why IN ('not_in_this_repo', 'skill_not_run')),
    skipped_name   TEXT,
    exit_code      INTEGER,
    block_on_fail  INTEGER NOT NULL CHECK (block_on_fail IN (0, 1)),
    repair_on_fail INTEGER NOT NULL CHECK (repair_on_fail IN (0, 1)),
    started_at     TEXT NOT NULL,
    ended_at       TEXT
) STRICT;
CREATE INDEX job_triggers_by_job ON job_triggers (job_id, firing_id);

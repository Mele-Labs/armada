-- Steps added to one Job, kept beside its frozen workflow and never in it.
-- docs/concepts/trigger.md. Additive. One row per addition; `state` is the
-- latest firing's and null until its moment has come, which is when it can
-- still be removed. A removed one keeps its row, `removed_at` set.
CREATE TABLE job_additions (
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
    state          TEXT CHECK (state IN ('skipped', 'running', 'passed', 'failed', 'awaiting_owner')),
    not_run_why    TEXT CHECK (not_run_why IN ('not_in_this_repo', 'skill_not_run', 'drone_step_not_run')),
    not_run_name   TEXT,
    exit_code      INTEGER,
    started_at     TEXT,
    ended_at       TEXT,
    kept           TEXT CHECK (kept IN ('repository', 'machine')),
    removed_at     TEXT,
    PRIMARY KEY (job_id, addition_id)
) STRICT;
CREATE INDEX job_additions_by_job ON job_additions (job_id, ordinal);

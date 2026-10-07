-- Which Job took each red main, for the hub's `fixing` and a Job's own mark. Additive.
CREATE TABLE main_ci_fixes (
    repository   TEXT NOT NULL,
    red_at       TEXT NOT NULL,
    job_id       TEXT NOT NULL,
    how          TEXT NOT NULL CHECK (how IN ('took', 'dispatched', 'sent_back')),
    check_name   TEXT NOT NULL,
    test         TEXT,
    merge_number INTEGER CHECK (merge_number IS NULL OR merge_number > 0),
    taken_at     TEXT NOT NULL,
    ended_at     TEXT,
    fixed_in     INTEGER CHECK (fixed_in IS NULL OR fixed_in > 0),
    PRIMARY KEY (repository, red_at, job_id)
) STRICT;

CREATE INDEX main_ci_fixes_by_job ON main_ci_fixes (job_id, taken_at);

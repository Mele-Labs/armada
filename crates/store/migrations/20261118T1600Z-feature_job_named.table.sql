-- The features a Job's commits named in a `Feature:` line, kept when Fleet's commit gate let the commit through, one
-- row a Job and feature. A Job owes sections only for these (docs/concepts/feature-catalog.md, *The hold*). `at` is
-- the first naming: a repeat keeps it. Additive.
CREATE TABLE feature_job_named (
    feature_id TEXT NOT NULL,
    job        TEXT NOT NULL,
    at         TEXT NOT NULL,
    PRIMARY KEY (feature_id, job)
) STRICT;

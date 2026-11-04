-- Which reading of the fold made a row: a Job whose row was made by an older one is split again by
-- the next pass, so a fix to the fold reaches the rows already kept and not only the Jobs to come.
-- Rows kept before this column existed read 0. Additive.
ALTER TABLE job_times ADD COLUMN fold INTEGER NOT NULL DEFAULT 0;

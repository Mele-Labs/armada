-- What Fleet has written back on the issue a Job came from (#91): whether it said the Job started,
-- the pull request it last named, the one label of Armada's the issue carries, and whether there is
-- nothing left to write. `failed` and `failed_at` are the last write the tracker refused.
CREATE TABLE job_issue_writeback (
    job_id       TEXT PRIMARY KEY REFERENCES jobs(job_id),
    started      INTEGER NOT NULL DEFAULT 0 CHECK (started IN (0, 1)),
    pull_request TEXT,
    label        TEXT CHECK (label IS NULL OR label IN ('in_progress', 'set_aside')),
    settled      INTEGER NOT NULL DEFAULT 0 CHECK (settled IN (0, 1)),
    failed       TEXT,
    failed_at    TEXT
) STRICT;

-- A Job that came from an issue before this is left alone: nothing is written on its issue.
INSERT INTO job_issue_writeback (job_id, settled) SELECT job_id, 1 FROM job_issue_sources;

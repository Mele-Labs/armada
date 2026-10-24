-- A Job the person renamed in Bridge, kept so the proposer names later Jobs the way he does.
-- One row a Job: `was` is the title before his first rename, `title` his latest, `request` the
-- Job's first message, `repository` its Manifest id. A second rename of one Job replaces `title`
-- and `renamed_at` only. Additive.
CREATE TABLE job_renames (
    session_id TEXT NOT NULL PRIMARY KEY,
    repository TEXT NOT NULL,
    request    TEXT NOT NULL,
    was        TEXT NOT NULL,
    title      TEXT NOT NULL,
    renamed_at TEXT NOT NULL
) STRICT;
CREATE INDEX job_renames_by_time ON job_renames (renamed_at);

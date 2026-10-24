-- The beads issue each Job worked, kept after the Job ends and its ledger row is given back, so
-- the merge of its pull request still closes the issue. `merged_in` is the address of the pull
-- request that merged, `closed_in` the one the issue was closed for: the two match once it is.
-- Additive.
CREATE TABLE job_issues (
    session_id  TEXT NOT NULL,
    manifest_id TEXT NOT NULL,
    issue       TEXT NOT NULL,
    merged_in   TEXT NOT NULL DEFAULT '',
    closed_in   TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (session_id, manifest_id, issue)
) STRICT;

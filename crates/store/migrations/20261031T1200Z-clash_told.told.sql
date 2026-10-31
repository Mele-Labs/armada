-- Each write-clash notice Fleet has sent a Job, by its words, so a restart that refills the clash
-- table from the worktrees does not send it again. The words name the other side and the files,
-- so a new set of files or a peer that is paused or resumed is told. Additive.
-- docs/concepts/fleet.md, *Write clashes*.
CREATE TABLE clash_told (
    session_id TEXT NOT NULL,
    said       TEXT NOT NULL,
    told_at    TEXT NOT NULL,
    PRIMARY KEY (session_id, said)
) STRICT;

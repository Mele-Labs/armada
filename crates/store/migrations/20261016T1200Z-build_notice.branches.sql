-- What a session was last told about each branch it holds, so a rebuild that leaves its branch where
-- it was says nothing. One row a (session, branch): the newest of the branch's commits the build held,
-- or `out`. Additive. docs/concepts/fleet.md, *Telling sessions the build they now run*.
CREATE TABLE build_branch_notices (
    recipient TEXT NOT NULL,
    branch    TEXT NOT NULL,
    standing  TEXT NOT NULL,
    noted_at  TEXT NOT NULL,
    PRIMARY KEY (recipient, branch)
) STRICT;

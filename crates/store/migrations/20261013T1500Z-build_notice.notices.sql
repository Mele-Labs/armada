-- What Fleet has already told a session about the build it now runs, so a restarted Fleet does not
-- tell it again. One row a (build commit, session). Additive. docs/concepts/fleet.md, *Telling
-- sessions the build they now run*.
CREATE TABLE build_notices (
    commit_id TEXT NOT NULL,
    recipient TEXT NOT NULL,
    noted_at  TEXT NOT NULL,
    PRIMARY KEY (commit_id, recipient)
) STRICT;

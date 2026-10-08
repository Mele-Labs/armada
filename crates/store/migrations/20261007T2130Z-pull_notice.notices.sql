-- What Fleet has already told a pull request's owner, so a restart does not tell it again, and
-- which pull requests the merge queue held when last read, so a drop out of it is a change.
-- Additive. docs/concepts/fleet.md, *Telling the owner of a pull request*.
CREATE TABLE pull_notices (
    repository TEXT NOT NULL,
    pull       INTEGER NOT NULL CHECK (pull > 0),
    head       TEXT NOT NULL,
    cause      TEXT NOT NULL,
    recipient  TEXT NOT NULL,
    noted_at   TEXT NOT NULL,
    PRIMARY KEY (repository, pull, head, cause, recipient)
) STRICT;

CREATE TABLE pull_queue_seen (
    repository TEXT NOT NULL,
    pull       INTEGER NOT NULL CHECK (pull > 0),
    PRIMARY KEY (repository, pull)
) STRICT;

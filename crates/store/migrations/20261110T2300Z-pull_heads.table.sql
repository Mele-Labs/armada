-- The commit each followed pull request was at when Fleet last read it, so a Job told its pull request
-- failed is known to have pushed nothing since: the notice kept in `pull_notices` names the commit it
-- was sent at, and this says the commit now. docs/concepts/session/what-a-job-is-doing.md.
CREATE TABLE pull_heads (
    repository TEXT NOT NULL,
    pull       INTEGER NOT NULL CHECK (pull > 0),
    head       TEXT NOT NULL,
    PRIMARY KEY (repository, pull)
) STRICT;

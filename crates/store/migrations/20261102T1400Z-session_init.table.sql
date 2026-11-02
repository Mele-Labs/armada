-- What a hosted Job's agent said it came up with on its opening line: the CLI's version, whether
-- the mod loaded, the state of Fleet's door and the names of its tools and skills, one name a line.
-- Only the harness's and the mod's names are written, so a person's own connectors are not here.
-- One row a session, replaced by each opening line. docs/concepts/session/a-job-that-came-up-without-its-tools.md.
CREATE TABLE session_init (
    session_id      TEXT NOT NULL PRIMARY KEY,
    harness_version TEXT NOT NULL,
    mod_loaded      INTEGER NOT NULL,
    door            TEXT NOT NULL,
    task_tools      INTEGER NOT NULL,
    tools           TEXT NOT NULL,
    skills          TEXT NOT NULL,
    at              TEXT NOT NULL
) STRICT;

-- What was said on top of a Job's For you, which Fleet otherwise reads off what it keeps. Additive.
-- docs/practices/protocol/for-you.md.
--
-- One row per Job and row id: its place in the order set (NULL where not placed), its line, and whether it was
-- withdrawn. A withdrawn id stays, so the item it names is not listed again.
CREATE TABLE for_you_marks (
    session_id TEXT NOT NULL,
    item       TEXT NOT NULL,
    place      INTEGER,
    line       TEXT,
    withdrawn  INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (session_id, item)
) STRICT;

-- The rows an agent added for what Fleet cannot see, `added:<n>`. `opens` is what a press opens (`artifact`,
-- `walk`, `waiting` or `plan`) and `target` its id or address, NULL for `plan`.
CREATE TABLE for_you_added (
    session_id TEXT NOT NULL,
    n          INTEGER NOT NULL,
    title      TEXT NOT NULL,
    kind       TEXT NOT NULL,
    opens      TEXT NOT NULL,
    target     TEXT,
    since      TEXT NOT NULL,
    PRIMARY KEY (session_id, n)
) STRICT;

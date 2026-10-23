-- The one pass that moved the open lessons that were repository facts into the Logbook
-- (docs/concepts/logbook.md). One row once it finished; none until then. `at` is milliseconds
-- since the epoch. No foreign key. Additive.
CREATE TABLE logbook_backlog_moved (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    at INTEGER NOT NULL
) STRICT;

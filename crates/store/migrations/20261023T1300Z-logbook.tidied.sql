-- When the Logbook was last tidied, per repository key (docs/concepts/logbook.md). `at` is
-- milliseconds since the epoch. No foreign key. Additive.
CREATE TABLE logbook_tidied (
    repository TEXT PRIMARY KEY,
    at INTEGER NOT NULL
) STRICT;

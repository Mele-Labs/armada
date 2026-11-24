-- The model calls his asks of a project's librarian spent on one night, a local date as `2026-11-24`: the count
-- `features.librarianAskShare` is held against, kept so a restart does not reset it. Additive.
-- docs/concepts/feature-catalog.md, *Librarian*.
CREATE TABLE librarian_ask_calls (
    project TEXT NOT NULL,
    night   TEXT NOT NULL,
    calls   INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (project, night)
) STRICT;

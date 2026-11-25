-- The parts of a project's system map that its librarian added, each kept apart until he keeps or removes it, and the
-- instant the librarian last mapped the project. Additive.
-- docs/practices/protocol/projects.md, *The system map*.
CREATE TABLE system_additions (
    project  TEXT NOT NULL,
    id       TEXT NOT NULL,
    name     TEXT NOT NULL,
    kind     TEXT NOT NULL,
    summary  TEXT NOT NULL,
    why      TEXT NOT NULL,
    -- One repository-relative path a line.
    paths    TEXT NOT NULL,
    manifest TEXT NOT NULL,
    -- `proposed`, `kept`, `rework` or `removed`.
    state    TEXT NOT NULL,
    note     TEXT NOT NULL DEFAULT '',
    added_at TEXT NOT NULL,
    PRIMARY KEY (project, id)
) STRICT;

CREATE TABLE system_librarian (
    project TEXT NOT NULL PRIMARY KEY,
    ran_at  TEXT NOT NULL
) STRICT;

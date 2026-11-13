-- What Fleet learned for a project rather than the person set: one row a (project, key), the word it
-- learned and when. The project's own file holds the value; this note says it came from his first answer,
-- so it is never in the file. Additive: a new table.
CREATE TABLE project_learned (
    project    TEXT NOT NULL,
    key        TEXT NOT NULL,
    word       TEXT NOT NULL,
    learned_at TEXT NOT NULL,
    PRIMARY KEY (project, key)
) STRICT;

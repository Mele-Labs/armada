-- How a Session was started, written once when its process first runs: what it held then, so a
-- later change of model, effort or mode does not rewrite it. The workflow is its name as it read
-- then. Additive: a new table; a Session started before it has no row.
CREATE TABLE session_started (
    session_id TEXT NOT NULL PRIMARY KEY,
    workflow   TEXT,
    delivery   TEXT,
    model      TEXT,
    effort     TEXT,
    mode       TEXT NOT NULL
) STRICT;

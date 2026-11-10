-- Where each project's file is kept: `this_mac`, `synced` or `cloud`, and what the place needs
-- besides its word, as `run_memory_places` keeps it for a repository. No row is no choice, and a
-- project with none is kept on this Mac. Never `repository`: a project is not kept in one. Additive.
CREATE TABLE project_places (
    project_id    TEXT PRIMARY KEY,
    place         TEXT NOT NULL,
    synced_folder TEXT,
    cloud         TEXT
) STRICT;

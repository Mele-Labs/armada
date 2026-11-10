-- The Workspace files a repository committed that Fleet turned into projects, by the path it read,
-- so a project he later deletes is not imported again. The file in the repository is never edited.
-- Additive.
CREATE TABLE project_imports (
    source     TEXT PRIMARY KEY,
    project_id TEXT NOT NULL
) STRICT;

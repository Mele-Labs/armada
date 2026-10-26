-- The projects a person keeps on this Mac, per repository, each an ordered list of rows. A row is a
-- task he wrote (`title`, `notes`, its own `state`) or a linked issue (`issue`, the tracker's
-- reference only: its state and title are the tracker's, read when the rows are). `state` is left
-- unchecked, as is `kind`, because a CHECK cannot be widened without a rebuild. Additive.
CREATE TABLE projects (
    id          TEXT PRIMARY KEY,
    manifest_id TEXT NOT NULL,
    name        TEXT NOT NULL,
    position    INTEGER NOT NULL,
    created     TEXT NOT NULL
) STRICT;
CREATE INDEX projects_by_repository ON projects (manifest_id, position);
CREATE TABLE project_rows (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    position   INTEGER NOT NULL,
    kind       TEXT NOT NULL,
    title      TEXT NOT NULL DEFAULT '',
    notes      TEXT NOT NULL DEFAULT '',
    issue      TEXT NOT NULL DEFAULT '',
    state      TEXT NOT NULL DEFAULT ''
) STRICT;
CREATE INDEX project_rows_by_project ON project_rows (project_id, position);

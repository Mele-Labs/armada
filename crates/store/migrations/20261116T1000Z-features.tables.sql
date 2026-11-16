-- A project's feature library, kept on this Mac (docs/concepts/feature-catalog.md). Areas hold features; a feature
-- owns path prefixes and holds the sections its Jobs wrote and the links Fleet detected. `project` is a project's id
-- and every id here is minted whole (never a rowid), so rows can be copied to another store unchanged when sharing is
-- built. Nothing here is shared with teammates now. Additive.
CREATE TABLE feature_areas (
    project  TEXT NOT NULL,
    slug     TEXT NOT NULL,
    title    TEXT NOT NULL,
    position INTEGER NOT NULL,
    PRIMARY KEY (project, slug)
) STRICT;

-- `state` is `seeded` or `confirmed`. `manifest_id` is the repository the prefixes are in, empty for any member of the
-- project. `entry_points` is a JSON array of `path` or `path:line`, the code a Job should open first.
CREATE TABLE features (
    id           TEXT PRIMARY KEY,
    project      TEXT NOT NULL,
    area         TEXT NOT NULL,
    slug         TEXT NOT NULL,
    title        TEXT NOT NULL,
    summary      TEXT NOT NULL,
    state        TEXT NOT NULL,
    manifest_id  TEXT NOT NULL DEFAULT '',
    entry_points TEXT NOT NULL DEFAULT '[]',
    position     INTEGER NOT NULL,
    UNIQUE (project, slug)
) STRICT;
CREATE INDEX features_by_project ON features (project, area, position);

CREATE TABLE feature_paths (
    feature_id TEXT NOT NULL,
    prefix     TEXT NOT NULL,
    PRIMARY KEY (feature_id, prefix)
) STRICT;

-- One current text per feature and `section` (`test`, `runs`, `observe` or `debt`): who wrote it, when, and the commit
-- the Job's checkout stood at.
CREATE TABLE feature_sections (
    feature_id TEXT NOT NULL,
    section    TEXT NOT NULL,
    text       TEXT NOT NULL,
    job        TEXT NOT NULL,
    written_at TEXT NOT NULL,
    at_commit  TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (feature_id, section)
) STRICT;

-- What Fleet saw tie a feature to work. `kind` is `job`, `pull_request`, `merge`, `sketch`, `page`, `walk`, `draft`,
-- `commit` or `bug`; `target` names the thing (a Job's id, a pull request's number, an artifact's path, a sha); `job` is
-- the Job it came through, empty where none; `via` is how it was found (`write`, `trailer`, `artifact`, `detector`).
CREATE TABLE feature_links (
    feature_id TEXT NOT NULL,
    kind       TEXT NOT NULL,
    target     TEXT NOT NULL,
    job        TEXT NOT NULL DEFAULT '',
    title      TEXT NOT NULL DEFAULT '',
    state      TEXT NOT NULL DEFAULT '',
    via        TEXT NOT NULL DEFAULT '',
    at         TEXT NOT NULL,
    PRIMARY KEY (feature_id, kind, target, job)
) STRICT;
CREATE INDEX feature_links_by_job ON feature_links (job, kind);

-- A file a Job wrote while its work was filed under a feature: its observed writes, and the files of its pull requests,
-- kept past a restart. A file's weight is the number of Jobs that wrote it.
CREATE TABLE feature_job_files (
    feature_id TEXT NOT NULL,
    job        TEXT NOT NULL,
    path       TEXT NOT NULL,
    first_at   TEXT NOT NULL,
    last_at    TEXT NOT NULL,
    PRIMARY KEY (feature_id, job, path)
) STRICT;
CREATE INDEX feature_job_files_by_job ON feature_job_files (job);

-- The files each merged pull request changed, per repository, so a section is read against the merges after it.
-- `job` is the Job that held the pull request, empty where none did. `files` is a JSON array of paths.
CREATE TABLE feature_merges (
    manifest_id TEXT NOT NULL,
    number      INTEGER NOT NULL,
    merged_at   TEXT NOT NULL,
    at_commit   TEXT NOT NULL DEFAULT '',
    job         TEXT NOT NULL DEFAULT '',
    files       TEXT NOT NULL,
    PRIMARY KEY (manifest_id, number)
) STRICT;
CREATE INDEX feature_merges_by_time ON feature_merges (manifest_id, merged_at);

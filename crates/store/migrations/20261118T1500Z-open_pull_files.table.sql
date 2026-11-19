-- The open pull requests Fleet read the lists of, with who opened each and the files it changes, one row a
-- pull request. `head_sha` is the commit the files were read at, so a head that has not moved is never read
-- again. `files` is a JSON array of paths, at most 100; `files_capped` says the pull request changes more.
-- docs/concepts/feature-catalog.md, *Who's on what*. Additive.
CREATE TABLE open_pull_files (
    manifest_id  TEXT    NOT NULL,
    number       INTEGER NOT NULL,
    repository   TEXT    NOT NULL,
    title        TEXT    NOT NULL,
    author       TEXT    NOT NULL,
    draft        INTEGER NOT NULL,
    head_sha     TEXT    NOT NULL,
    files        TEXT    NOT NULL,
    files_capped INTEGER NOT NULL,
    read_at      TEXT    NOT NULL,
    PRIMARY KEY (manifest_id, number)
) STRICT;

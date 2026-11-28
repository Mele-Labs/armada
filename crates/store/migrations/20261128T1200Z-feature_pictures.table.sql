-- The pictures of what a feature looks like in the running app, one row a picture. The file is Fleet's, under its support
-- folder at `feature-pictures/<file>`; the row keeps where it came from: `url` for a web page, `screen` for a native screen
-- that has no address (a walk's name), the commit the app was built from, when it was taken, its size in pixels and bytes,
-- `job` the Job that took it (empty for the librarian) and `taken_by` (`job` or `librarian`). `removed_at` is his "Not this
-- feature", with `removed_why` what he gave, empty where nothing; a removed row stays, so what he taught is kept. Additive.
-- docs/concepts/feature-catalog.md, *Pictures*.
CREATE TABLE feature_pictures (
    id          TEXT PRIMARY KEY,
    feature_id  TEXT NOT NULL,
    project     TEXT NOT NULL,
    file        TEXT NOT NULL,
    url         TEXT NOT NULL DEFAULT '',
    screen      TEXT NOT NULL DEFAULT '',
    at_commit   TEXT NOT NULL DEFAULT '',
    taken_at    TEXT NOT NULL,
    width       INTEGER NOT NULL,
    height      INTEGER NOT NULL,
    bytes       INTEGER NOT NULL,
    job         TEXT NOT NULL DEFAULT '',
    taken_by    TEXT NOT NULL,
    removed_at  TEXT,
    removed_why TEXT NOT NULL DEFAULT ''
) STRICT;

CREATE INDEX feature_pictures_by_feature ON feature_pictures (feature_id, taken_at);

-- What the person remembers of a pull request between visits, and the Reviews layouts he named.
-- `pull_request_seen`: the head he last left a pull request at. `pull_request_viewed`: a file he
-- marked viewed, at the commit he read and the forge's blob for it then (NULL where none was given).
-- `review_layouts`: a pane-tree arrangement as opaque JSON, by name; `review_layout_picked` holds
-- the one row naming the layout every pull request opens on. Additive.
-- docs/concepts/pull-review.md, *Since you looked*.
CREATE TABLE pull_request_seen (
    manifest_id TEXT    NOT NULL,
    repository  TEXT    NOT NULL,
    number      INTEGER NOT NULL,
    head        TEXT    NOT NULL,
    seen_at     TEXT    NOT NULL,
    PRIMARY KEY (manifest_id, repository, number)
) STRICT;

CREATE TABLE pull_request_viewed (
    manifest_id TEXT    NOT NULL,
    repository  TEXT    NOT NULL,
    number      INTEGER NOT NULL,
    path        TEXT    NOT NULL,
    commit_sha  TEXT    NOT NULL,
    blob        TEXT,
    viewed_at   TEXT    NOT NULL,
    PRIMARY KEY (manifest_id, repository, number, path)
) STRICT;

CREATE TABLE review_layouts (
    name        TEXT NOT NULL PRIMARY KEY,
    arrangement TEXT NOT NULL,
    saved_at    TEXT NOT NULL
) STRICT;

CREATE TABLE review_layout_picked (
    id   INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
    name TEXT    NOT NULL
) STRICT;

-- Where changes to one area of a repository run, kept so the next pull request there does not ask:
-- how to reach it (`deployed` or `pull`), its web address, the command that starts a pulled-down
-- change, and its API base. One place a repository and area. Additive.
-- docs/concepts/pull-review.md, *Walk*.
CREATE TABLE walk_places (
    id          TEXT NOT NULL PRIMARY KEY,
    manifest_id TEXT NOT NULL,
    area        TEXT NOT NULL,
    how         TEXT NOT NULL,
    address     TEXT NOT NULL,
    command     TEXT NOT NULL,
    api         TEXT NOT NULL,
    saved_at    TEXT NOT NULL,
    UNIQUE (manifest_id, area)
) STRICT;

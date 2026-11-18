-- The Reviews lists Fleet last read, one row a project (the empty string is every project), so a Fleet that
-- starts answers Reviews at once and reads the forge behind it. `lists` is the wire's JSON, opaque here.
-- docs/practices/protocol/the-reviews-tab-the-lists-waiting-on-a-person-a-pages-threads-and-a.md. Additive.
CREATE TABLE review_lists_kept (
    project    TEXT NOT NULL PRIMARY KEY,
    fetched_at TEXT NOT NULL,
    lists      TEXT NOT NULL
) STRICT;

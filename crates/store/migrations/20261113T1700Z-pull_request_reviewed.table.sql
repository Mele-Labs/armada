-- The review Fleet last wrote for the person on a pull request: what it said, the commit it was
-- written on and when Fleet wrote it. The forge can answer a read just after a write without the
-- review, so Fleet serves this one until the forge shows one as new. Additive.
-- docs/concepts/pull-review.md, *Your review*.
CREATE TABLE pull_request_reviewed (
    manifest_id  TEXT    NOT NULL,
    repository   TEXT    NOT NULL,
    number       INTEGER NOT NULL,
    said         TEXT    NOT NULL,
    commit_sha   TEXT,
    submitted_at TEXT    NOT NULL,
    PRIMARY KEY (manifest_id, repository, number)
) STRICT;

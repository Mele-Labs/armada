-- The last time a Bridge was served a pull request's page, which is when he opened it: a review asked of
-- him counts as opened once he has read the page since his review before the request. Additive.
-- docs/practices/protocol/a-jobs-calls.md, *A review asked of him*.
CREATE TABLE pull_requests_opened (
    manifest_id TEXT    NOT NULL,
    number      INTEGER NOT NULL,
    opened_at   TEXT    NOT NULL,
    PRIMARY KEY (manifest_id, number)
) STRICT;

-- A review asked of the person by name that Fleet has told him of, so a restart does not tell it again.
-- One row a request: the pull request, and when his latest review before it was submitted ('' for none),
-- since the forge drops him from the request when he reviews and a request after that is a new one. Additive.
-- docs/practices/protocol/a-jobs-calls.md, *A review asked of him*.
CREATE TABLE review_requests_told (
    repository   TEXT    NOT NULL,
    number       INTEGER NOT NULL,
    after_review TEXT    NOT NULL,
    told_at      TEXT    NOT NULL,
    PRIMARY KEY (repository, number, after_review)
) STRICT;

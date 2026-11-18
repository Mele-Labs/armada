-- A pull request the person ignored in the Reviews inbox, and when. Fleet lifts it by itself once he
-- is asked of it again after `ignored_at`. Kept on his Mac, never on the forge. Additive.
-- docs/practices/protocol/what-fleet-remembers-of-a-pull-request.md.
CREATE TABLE pull_request_ignored (
    manifest_id TEXT    NOT NULL,
    repository  TEXT    NOT NULL,
    number      INTEGER NOT NULL,
    ignored_at  TEXT    NOT NULL,
    PRIMARY KEY (manifest_id, repository, number)
) STRICT;

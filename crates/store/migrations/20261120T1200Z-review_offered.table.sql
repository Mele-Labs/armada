-- The open pull requests Fleet has looked at in a repository, so one is offered review tools once: a row a
-- (manifest, pull request), kept for good and across a restart. `number` 0 is the repository's baseline: the first look
-- keeps what was open then and offers nothing. Additive. docs/practices/protocol/projects.md, *Review on a new pull request*.
CREATE TABLE review_offered (
    manifest_id TEXT    NOT NULL,
    number      INTEGER NOT NULL,
    seen_at     TEXT    NOT NULL,
    PRIMARY KEY (manifest_id, number)
) STRICT;

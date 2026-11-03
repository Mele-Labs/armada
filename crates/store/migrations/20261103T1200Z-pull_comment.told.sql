-- Each comment on a pull request a Job was told of, with its edit time when told, so a comment
-- is told again only once it is edited and a restart tells nobody twice. A row with an empty
-- `comment` marks a pull request read with nothing said on it, so the first comment after is new
-- rather than a take-on. The recipient is plain text (`session:<id>`) and no foreign key, as in
-- `pull_notices`. Additive. docs/concepts/fleet.md, *Telling the owner of a pull request*.
CREATE TABLE pull_comments_told (
    repository TEXT NOT NULL,
    pull       INTEGER NOT NULL CHECK (pull > 0),
    recipient  TEXT NOT NULL,
    comment    TEXT NOT NULL,
    edited_at  TEXT,
    told_at    TEXT NOT NULL,
    PRIMARY KEY (repository, pull, recipient, comment)
) STRICT;

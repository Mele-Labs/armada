-- A Fleet notice a person turned off, for one job, for a repository, or until a time. Kept so a Fleet
-- that restarts still holds its tongue. One row a (kind, scope, key), replaced by the next mute. Additive.
CREATE TABLE notice_mutes (
    kind  TEXT NOT NULL,
    scope TEXT NOT NULL,
    key   TEXT NOT NULL,
    until TEXT,
    PRIMARY KEY (kind, scope, key)
) STRICT;

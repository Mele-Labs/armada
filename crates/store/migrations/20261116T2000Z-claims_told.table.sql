-- Each claim a Job was told another holds, by what it is on and who holds it, so a restart, which
-- forgets the claims Fleet keeps in memory, tells the Job nothing again. Additive.
-- docs/concepts/fleet.md, *Claims a Job holds*.
CREATE TABLE claims_told (
    session_id TEXT NOT NULL,
    about      TEXT NOT NULL,
    holder     TEXT NOT NULL,
    told_at    TEXT NOT NULL,
    PRIMARY KEY (session_id, about, holder)
) STRICT;

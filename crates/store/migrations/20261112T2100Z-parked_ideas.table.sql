-- An idea the person's dispatch was kept as, for later: the whole body of his `dispatch_job`, opaque here, until he
-- starts a Job on it or removes it. Kept by Fleet, since a Bridge that is closed when the Operator parks it has no
-- list to put it in. Additive.
CREATE TABLE parked_ideas (
    id         TEXT NOT NULL PRIMARY KEY,
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

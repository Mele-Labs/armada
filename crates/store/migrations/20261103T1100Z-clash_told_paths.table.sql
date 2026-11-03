-- Each file of a write clash a Job was told about, by the other Job, so a later commit that adds
-- another file to the overlap names only that file and never one already told. Additive.
-- docs/concepts/fleet.md, *Write clashes*.
CREATE TABLE clash_told_paths (
    session_id TEXT NOT NULL,
    peer_id    TEXT NOT NULL,
    path       TEXT NOT NULL,
    told_at    TEXT NOT NULL,
    PRIMARY KEY (session_id, peer_id, path)
) STRICT;

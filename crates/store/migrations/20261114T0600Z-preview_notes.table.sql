-- What a Job tells the owner to look at when it offers a branch for the preview: `look` is one line and `steps` a
-- JSON array of `{say, act?}`, kept on the offer so a re-offer that leaves them out keeps them. Additive.
ALTER TABLE preview_offered ADD COLUMN look TEXT;
ALTER TABLE preview_offered ADD COLUMN steps TEXT NOT NULL DEFAULT '[]';

-- The note of one branch at one tip, kept for the build the owner runs. **It outlives the offer**: an offer is
-- dropped when its Job ends or its tip lands, and the build he loaded still holds the branch. One row a
-- (branch, tip), so a new tip is a new note, not looked at. `looked_at` is when he marked it. No foreign key. Additive.
CREATE TABLE build_notes (
    branch     TEXT NOT NULL,
    tip        TEXT NOT NULL,
    session_id TEXT NOT NULL,
    about      TEXT NOT NULL,
    look       TEXT,
    steps      TEXT NOT NULL DEFAULT '[]',
    offered_at TEXT NOT NULL,
    looked_at  TEXT,
    PRIMARY KEY (branch, tip)
) STRICT;

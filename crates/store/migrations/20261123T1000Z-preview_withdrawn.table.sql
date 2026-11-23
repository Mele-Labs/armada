-- The (branch, tip) pairs a Job took back with `withdraw_preview`, so the next Refresh preview leaves that tip out
-- though the running build holds it. A new offer of the branch forgets them. Additive.
-- docs/concepts/fleet.md, *Offering a branch for the preview*.
CREATE TABLE preview_withdrawn (
    branch       TEXT NOT NULL,
    tip          TEXT NOT NULL,
    withdrawn_at TEXT NOT NULL,
    PRIMARY KEY (branch, tip)
) STRICT;

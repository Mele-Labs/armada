-- When a walk was last shown for a plan group, so a group whose `walk_approved` gate closed is open
-- again once a newer walk is shown for it. The close rows are never edited: the gate reads its newest
-- close against this.
CREATE TABLE session_gate_walks (
    session_id TEXT NOT NULL,
    group_id   TEXT NOT NULL,
    shown_at   TEXT NOT NULL,
    PRIMARY KEY (session_id, group_id)
);

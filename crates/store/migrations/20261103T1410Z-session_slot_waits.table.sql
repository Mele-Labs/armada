-- How long a Job waited for a slot, in two phases: `line`, in the repository's line while every
-- slot was leased, and `move`, from the slot being leased for it to Fleet putting the Job in it.
-- Two stamps a wait, `ended_at` empty while it stands. The timeline enters only the lease and the
-- release, and prunes after 30 days; this is kept for good (docs/concepts/session.md). Additive.
CREATE TABLE session_slot_waits (
    session_id TEXT NOT NULL,
    phase      TEXT NOT NULL CHECK (phase IN ('line', 'move')),
    began_at   TEXT NOT NULL,
    ended_at   TEXT,
    PRIMARY KEY (session_id, phase, began_at)
) STRICT, WITHOUT ROWID;

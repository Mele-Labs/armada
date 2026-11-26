-- The order a Job set on its open asks with `order_waiting`, and why each stands there. Additive.
-- docs/concepts/session/waiting-on-the-person.md, *What he needs first*.
--
-- One row per Job and item id, `place` from 0. Every call replaces the Job's rows, and an id whose item is settled
-- is left off the read, not deleted.
CREATE TABLE waiting_order (
    session_id TEXT NOT NULL,
    item       TEXT NOT NULL,
    place      INTEGER NOT NULL,
    why        TEXT NOT NULL,
    PRIMARY KEY (session_id, item)
) STRICT;

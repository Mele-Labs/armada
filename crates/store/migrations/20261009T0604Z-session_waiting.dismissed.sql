-- The items the person dismissed for good from what a session waits on, by item id. A derived item
-- (an open card, a permission, a walk window) with a dismissed id is never listed again, and an
-- agent item with one is dropped from the kept list. docs/concepts/session.md.
CREATE TABLE session_waiting_dismissed (
    session_id TEXT NOT NULL,
    item_id    TEXT NOT NULL,
    PRIMARY KEY (session_id, item_id)
) STRICT;

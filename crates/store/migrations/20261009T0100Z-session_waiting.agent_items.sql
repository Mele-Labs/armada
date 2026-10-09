-- What a session's agent said it is waiting on the person for, set whole by the
-- agent's tool. Fleet's own items (an open card, an unapproved walk) are
-- derived on read and never kept here. `position` keeps the agent's order.
-- docs/concepts/session.md.
CREATE TABLE session_waiting (
    session_id TEXT NOT NULL,
    position   INTEGER NOT NULL,
    item_id    TEXT NOT NULL,
    text       TEXT NOT NULL,
    -- When this id first began to wait, RFC3339; kept across updates.
    since      TEXT NOT NULL,
    act_kind   TEXT,
    act_target TEXT,
    -- A JSON array of labels; empty when the item has no choices.
    options    TEXT NOT NULL,
    PRIMARY KEY (session_id, item_id)
) STRICT;

-- An agent's item that asks for his approval: 1 where it does. Kept on the open item and on the closed one,
-- so a restart and a reopen keep its two options. Additive: a row kept before this reads as not asking.
-- `session_waiting_approvals` keeps each Approve he pressed on such an item, which a `you_approved` gate
-- closes on. Never pruned and never undone by a reopen: the gate reads it once. docs/concepts/session.md.
ALTER TABLE session_waiting ADD COLUMN approves INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_waiting_dismissed ADD COLUMN approves INTEGER NOT NULL DEFAULT 0;
CREATE TABLE session_waiting_approvals (
    session_id TEXT NOT NULL,
    item_id    TEXT NOT NULL,
    -- The item's text as he approved it.
    text       TEXT NOT NULL,
    -- When he pressed Approve, RFC3339.
    at         TEXT NOT NULL,
    PRIMARY KEY (session_id, item_id, at)
) STRICT;

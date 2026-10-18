-- A session's Loadout (docs/practices/protocol.md, *A Session's Loadout*): the latest list of what its
-- agent knows and was told, one row a session replaced at each `equipped`, and what it used, one row a
-- use. Both texts are the wire's own JSON (ipc::LoadoutItems, ipc::LoadoutUse), opaque to the store.
-- A use reported twice in one turn is one row. Uses are trimmed to a session's last fifty turns. No
-- foreign key, as session_context has none. Additive.
CREATE TABLE session_loadout (
    session_id TEXT NOT NULL PRIMARY KEY,
    items      TEXT NOT NULL
) STRICT;

CREATE TABLE session_loadout_uses (
    seq             INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      TEXT NOT NULL,
    turn_started_at TEXT NOT NULL,
    used            TEXT NOT NULL,
    UNIQUE (session_id, turn_started_at, used)
) STRICT;

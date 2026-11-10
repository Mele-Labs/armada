-- The person's Your turn queue: what waits on him, which the Operator reviewed first, in the order he
-- left it. One row for each item, kept as it was raised, whatever its Job's list does after.
-- `session` is the Job whose list holds the item, empty where no Job does; `body` is the item as
-- `ipc::WaitingItem` JSON; `read_text`, `pick` and `then_pick` are what the Operator made of it.
-- `go_ahead` marks what the Operator would do itself. `ord` is the order he dragged; `later_until`
-- puts the row last until then. A row is open until `settled_at`; `answer` is the last press on it.
-- `operator_page` holds the page's one row: when the window was last seen and last looked, and the
-- top line the Operator wrote. Additive.
CREATE TABLE operator_turn (
    n INTEGER PRIMARY KEY AUTOINCREMENT,
    session TEXT NOT NULL,
    item TEXT NOT NULL,
    manifest_id TEXT NOT NULL,
    job TEXT,
    go_ahead INTEGER NOT NULL,
    body TEXT NOT NULL,
    read_text TEXT,
    pick TEXT,
    then_pick TEXT,
    ord INTEGER NOT NULL,
    raised_at TEXT NOT NULL,
    later_until TEXT,
    answer TEXT,
    settled_at TEXT,
    call TEXT
);
CREATE UNIQUE INDEX operator_turn_open ON operator_turn (session, item) WHERE settled_at IS NULL;
CREATE TABLE operator_page (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    seen_at TEXT,
    looked_at TEXT,
    top TEXT,
    top_at TEXT
);

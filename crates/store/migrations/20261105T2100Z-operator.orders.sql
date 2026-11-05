-- What the person typed to the Operator and what it said back, and the standing orders he confirmed.
-- `form` is `ipc::operator_talk::OperatorOrderForm` as JSON, one of the forms Fleet enforces, and
-- `until` the instant an order for today stops standing. `removed_at` is null while it stands. A
-- row's `call` is the call that answered it, which the day's ceiling counts. Additive.
CREATE TABLE operator_orders (
    n          INTEGER PRIMARY KEY AUTOINCREMENT,
    form       TEXT NOT NULL,
    said       TEXT NOT NULL,
    until      TEXT,
    at         TEXT NOT NULL,
    removed_at TEXT
);

CREATE TABLE operator_talk (
    n    INTEGER PRIMARY KEY AUTOINCREMENT,
    from_person INTEGER NOT NULL,
    text TEXT NOT NULL,
    call TEXT,
    at   TEXT NOT NULL
);

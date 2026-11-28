-- What the person sent the Operator while no session of it ran, kept until one is up and told it.
-- `talk` is `ipc::TalkToOperator` as JSON, as he sent it; `delivered_at` is null until the session
-- that came up was told it. Additive.
CREATE TABLE operator_held_sends (
    n            INTEGER PRIMARY KEY AUTOINCREMENT,
    talk         TEXT NOT NULL,
    at           TEXT NOT NULL,
    delivered_at TEXT
);

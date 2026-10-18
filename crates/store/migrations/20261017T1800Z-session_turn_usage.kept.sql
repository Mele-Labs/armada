-- What a session's turns cost, a row a turn (docs/concepts/session.md, *Where the tokens went*):
-- the model, the four token fields and the names of the tools the turn called, so usage can be read
-- by session, model and tool over a window. A turn is named by the message id its harness gave it,
-- and is one row however often it is reported. `tools` is a JSON array of names, never inputs. `at`
-- is RFC3339 as the reporter stamped it. No foreign key, as session_context has none. Additive.
CREATE TABLE session_turn_usage (
    session_id            TEXT NOT NULL,
    message_id            TEXT NOT NULL,
    at                    TEXT NOT NULL,
    model                 TEXT NOT NULL,
    input_tokens          INTEGER NOT NULL,
    cache_creation_tokens INTEGER NOT NULL,
    cache_read_tokens     INTEGER NOT NULL,
    output_tokens         INTEGER NOT NULL,
    tools                 TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY (session_id, message_id)
) STRICT, WITHOUT ROWID;

CREATE INDEX session_turn_usage_by_time ON session_turn_usage (session_id, at);

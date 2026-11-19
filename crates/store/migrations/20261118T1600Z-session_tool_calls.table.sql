-- Each tool call of a thread, for the waste that comes of what calls put into the context
-- (docs/concepts/session/where-the-tokens-went.md, *Waste*): a file read again, and a result over 20 KB.
-- **No text is kept**: a call is its tool's name, a Read's file path, and its result's size in bytes.
-- `thread` is empty for the session's own and a subagent's file name for a subagent's, which has its own
-- context. `epoch` is the compactions the thread had had, so a file read after one is not a repeat.
-- `result_bytes` is absent until the result is read, and `carried_by` names the first request of the thread
-- after the result, the one that paid for it. `tooled` marks a transcript read by a build that keeps calls,
-- so one that was not is read again from the top, once. Additive.
CREATE TABLE session_tool_calls (
    session_id   TEXT NOT NULL,
    tool_use_id  TEXT NOT NULL,
    message_id   TEXT NOT NULL,
    at           TEXT NOT NULL,
    thread       TEXT NOT NULL,
    epoch        INTEGER NOT NULL,
    tool         TEXT NOT NULL,
    path         TEXT,
    result_bytes INTEGER,
    carried_by   TEXT,
    PRIMARY KEY (session_id, tool_use_id)
) STRICT, WITHOUT ROWID;

CREATE INDEX session_tool_calls_by_carrier ON session_tool_calls (session_id, carried_by);
CREATE INDEX session_tool_calls_by_path ON session_tool_calls (session_id, thread, epoch, path) WHERE path IS NOT NULL;

ALTER TABLE transcript_offsets ADD COLUMN tooled INTEGER NOT NULL DEFAULT 0;

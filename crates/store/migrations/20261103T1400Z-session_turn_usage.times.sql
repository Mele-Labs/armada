-- When each turn waited on the model and ran its tools, read from the transcript's own stamps, so
-- a Job's time can be split by where it went (docs/concepts/session.md). `model_began` is the user
-- line the message answered, `tools_began` the first call written and `tools_ended` the last result
-- read for its calls: wall time, so two calls that ran together are one stretch. All three stay
-- empty on a turn kept before this, which no later read fills unless its file is read again from
-- the top. Stamps only, never a call's input or a result's content. Additive.
ALTER TABLE session_turn_usage ADD COLUMN model_began TEXT;
ALTER TABLE session_turn_usage ADD COLUMN tools_began TEXT;
ALTER TABLE session_turn_usage ADD COLUMN tools_ended TEXT;

-- A call whose result the transcript had not written when it was read: the result arrives in a
-- later read, which has the call's id and nothing else of it. Removed when the result lands.
CREATE TABLE session_open_tools (
    session_id  TEXT NOT NULL,
    tool_use_id TEXT NOT NULL,
    message_id  TEXT NOT NULL,
    PRIMARY KEY (session_id, tool_use_id)
) STRICT, WITHOUT ROWID;

-- The last user line a transcript read reached, so the first message of the next read knows when
-- its model began.
ALTER TABLE transcript_offsets ADD COLUMN input_at TEXT;

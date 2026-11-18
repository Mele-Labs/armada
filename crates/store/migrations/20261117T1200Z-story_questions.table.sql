-- A message of his sorted as a question, and the reply of the Job's that answers it. One row a question, keyed as
-- its `story_messages` row is; his words and when he asked are read from there. `due` is set when the Job's turn
-- ends, or when the message is sorted as a question, and taken by the notice loop's read. `checked_row` is the
-- last reply read as not answering, so the same reply is not asked about twice. `reply_row`, `excerpt` and
-- `answered_at` are set together when a reply answers, and `read_at` when he read that answer. Additive.
CREATE TABLE story_questions (
    session_id  TEXT NOT NULL,
    row_id      TEXT NOT NULL,
    due         INTEGER NOT NULL,
    checked_row TEXT,
    reply_row   TEXT,
    excerpt     TEXT,
    answered_at TEXT,
    read_at     TEXT,
    PRIMARY KEY (session_id, row_id)
) STRICT;
CREATE INDEX story_questions_due ON story_questions (due);

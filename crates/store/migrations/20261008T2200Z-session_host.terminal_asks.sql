-- A terminal session's question, kept until its mod has collected the answer.
-- Its mod polls for the answer across connections, so the question and an
-- answer from Bridge outlive any one request and a Fleet restart. One a
-- session: a new question replaces the last. docs/concepts/session.md.
CREATE TABLE terminal_asks (
    session_id TEXT NOT NULL PRIMARY KEY,
    call       TEXT NOT NULL,
    -- The tool's input as the mod sent it, and the call as Bridge draws it.
    asking     TEXT NOT NULL,
    in_flight  TEXT NOT NULL,
    -- The `TerminalAsked` the mod will be handed; null while nobody answered.
    answer     TEXT
) STRICT;

-- What fills a session's context window, as the session's mod last reported it. One row a session,
-- replaced at each report; the text is the wire's own JSON (ipc::ContextBreakdown), opaque to the
-- store. A table of its own because keep_session replaces the whole sessions row. docs/concepts/session.md.
CREATE TABLE session_context (
    session_id TEXT NOT NULL PRIMARY KEY,
    breakdown  TEXT NOT NULL
) STRICT;

-- The last Fleet tell a Session was sent that its Job may turn down in prose: which, when, and the first
-- reply that followed it. One row a session, replaced by the next tell. Additive.
-- docs/spikes/tell-refusal-reader.md.
CREATE TABLE session_tells (
    session_id TEXT NOT NULL PRIMARY KEY,
    kind       TEXT NOT NULL,
    at         TEXT NOT NULL,
    first_end  TEXT
) STRICT;

-- The last instant the person was in a Job, as a client told it (`view_session`): RFC 3339 on Fleet's
-- clock, kept later only. The row mark reads a reply as read when this is at or past the Job's last
-- turn. No foreign key, as `session_focus` has none. Additive.
CREATE TABLE session_viewed (
    session_id TEXT NOT NULL PRIMARY KEY,
    viewed_at  TEXT NOT NULL
) STRICT;

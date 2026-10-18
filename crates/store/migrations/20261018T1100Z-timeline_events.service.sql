-- The service a `session.called` entry records a call to (`docs/practices/protocol.md`, *The Fleet's timeline*).
-- The call's line is the entry's `detail`. Null for every other kind and for an entry written before this
-- column. Additive.
ALTER TABLE timeline_events ADD COLUMN service TEXT;

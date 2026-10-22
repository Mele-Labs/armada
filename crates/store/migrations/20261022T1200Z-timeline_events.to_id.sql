-- The Session a `session.messaged` entry's message was delivered to; the entry's subject sent it.
-- Null for every other kind and for an entry written before this column. Additive.
ALTER TABLE timeline_events ADD COLUMN to_id TEXT;

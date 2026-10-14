-- The files or commands a burst on the timeline counted (`Read 14 files`), newest first, one to a line.
-- Null for an entry written before this column and for every kind that counts nothing. Additive.
ALTER TABLE timeline_events ADD COLUMN items TEXT;

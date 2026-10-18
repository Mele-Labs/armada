-- What a `session.called` entry calls its service (`GitHub`, `the web`, a server's own name), beside `service`.
-- Null for every other kind and for an entry written before this column. Additive.
ALTER TABLE timeline_events ADD COLUMN service_name TEXT;

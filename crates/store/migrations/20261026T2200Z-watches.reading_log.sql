-- A failed or broken reading's whole output, kept as a file under Fleet's state: its path, or
-- absent where none was kept. Additive.
ALTER TABLE watch_readings ADD COLUMN log TEXT;

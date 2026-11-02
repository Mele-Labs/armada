-- A Mod Job started with the aim `instrument` builds an instrument for a bay of the console, and
-- the record keeps that, so Bridge finds the Job on its bay after a restart. Additive: 0 on every other row.
ALTER TABLE hosted_sessions ADD COLUMN mod_of_bay INTEGER NOT NULL DEFAULT 0;

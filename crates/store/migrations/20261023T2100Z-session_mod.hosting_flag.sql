-- A hosted session started from Mod: a Mod Job, for modding Armada itself. 1 is one. Additive: the
-- column defaults to 0.
ALTER TABLE hosted_sessions ADD COLUMN mod_job INTEGER NOT NULL DEFAULT 0;

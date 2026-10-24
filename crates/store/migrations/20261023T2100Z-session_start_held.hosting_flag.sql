-- A hosted session that waits on its start card: no agent runs and a message is kept, until the
-- person presses Start. 1 is held. Additive: the column defaults to 0.
ALTER TABLE hosted_sessions ADD COLUMN start_held INTEGER NOT NULL DEFAULT 0;

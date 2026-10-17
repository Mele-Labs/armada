-- A hosted session the person paused: its slot is given back and its work is parked on
-- `lease_branch`. `paused_at` is when (NULL is not paused); `paused_resuming` is 1 while a
-- resume waits for a slot. Additive: every column is NULL or defaults to 0.
ALTER TABLE hosted_sessions ADD COLUMN paused_at TEXT;
ALTER TABLE hosted_sessions ADD COLUMN paused_resuming INTEGER NOT NULL DEFAULT 0;

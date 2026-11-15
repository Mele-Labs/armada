-- The Operator is one hosted session per Mac, and the row says which it is, so a restart and a hand
-- on to a successor find it again. Additive: 0 on every other row.
ALTER TABLE hosted_sessions ADD COLUMN operator INTEGER NOT NULL DEFAULT 0;

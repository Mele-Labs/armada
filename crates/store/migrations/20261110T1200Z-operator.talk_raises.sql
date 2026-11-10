-- The Operator raising a Needs you item to the person in its conversation, and its briefing of the day.
-- `raised` is `ipc::operator_talk::OperatorRaise` as JSON, without `settled_at`, and `raise_session`
-- and `raise_item` name the item it raised (the session empty for a suggestion no Job holds).
-- `settled_at` is when the item left the list, null while it waits. `briefing` marks the day's
-- briefing, and the newest one's `at` is the day last briefed. Additive.
ALTER TABLE operator_talk ADD COLUMN raised TEXT;
ALTER TABLE operator_talk ADD COLUMN raise_session TEXT;
ALTER TABLE operator_talk ADD COLUMN raise_item TEXT;
ALTER TABLE operator_talk ADD COLUMN settled_at TEXT;
ALTER TABLE operator_talk ADD COLUMN briefing INTEGER NOT NULL DEFAULT 0;
CREATE INDEX operator_talk_raises_open ON operator_talk (raise_session, raise_item) WHERE raised IS NOT NULL AND settled_at IS NULL;

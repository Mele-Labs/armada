-- The page a recorded plan depends on (docs/concepts/session.md, *The plan*). Its call waits until the
-- person approves that walk. Null where the plan names none, and for every plan recorded before this
-- column. Additive.
ALTER TABLE session_plan ADD COLUMN walk TEXT;

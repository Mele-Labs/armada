-- The judge type a reading read as, by id and by the name it had then. NULL on a reading kept before judge
-- types, which read as adversarial. Additive. docs/concepts/judge.md, *Judge types*.
ALTER TABLE judge_readings ADD COLUMN judge TEXT;
ALTER TABLE judge_readings ADD COLUMN judge_name TEXT;

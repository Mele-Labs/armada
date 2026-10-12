-- What a panel member said makes refused work risky, where it said so. Null on a member that
-- said it only needs redoing, on a Judge of one, and on every row written before this. #206,
-- docs/concepts/judge.md.
ALTER TABLE job_step_judgments ADD COLUMN risky TEXT;

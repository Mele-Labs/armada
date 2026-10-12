-- The question a Job's Drone asked, held on the Job after that Drone stands down, and the answer
-- until the next Drone opens with it. Null on every Job with nothing asked. `question_id` is the
-- question's presence; `question_options` is a JSON array of [label, consequence] pairs.
ALTER TABLE jobs ADD COLUMN question_id TEXT;
ALTER TABLE jobs ADD COLUMN question_step TEXT;
ALTER TABLE jobs ADD COLUMN question_asked_at TEXT;
ALTER TABLE jobs ADD COLUMN question_text TEXT;
ALTER TABLE jobs ADD COLUMN question_options TEXT;
ALTER TABLE jobs ADD COLUMN question_chose TEXT;
ALTER TABLE jobs ADD COLUMN question_answered_at TEXT;

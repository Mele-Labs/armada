-- The several decisions a Drone asked in one ask, on the Job beside the question. A JSON array of
-- {id, question, options: [[label, consequence]], chose}. Null on a single question and on a Job
-- with nothing asked; `question_text` is empty and `question_options` is `[]` where this is set.
ALTER TABLE jobs ADD COLUMN question_decisions TEXT;

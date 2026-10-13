-- The line a person's best or quick left with the answer: told to the next Drone, which decides for
-- itself. Null where the person picked an offered answer and added nothing. With a mode alone,
-- `question_chose` is null too, and `question_answered_at` is the answer's presence.
ALTER TABLE jobs ADD COLUMN question_deciding TEXT;
